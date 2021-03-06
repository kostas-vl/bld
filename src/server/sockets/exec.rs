use crate::config::BldConfig;
use crate::helpers::term;
use crate::path;
use crate::persist::{Database, FileLogger, FileScanner, Scanner};
use crate::run::{Pipeline, Runner};
use crate::server::{PipelinePool, User};
use crate::types::{BldError, ExecInfo, Result};
use actix::prelude::*;
use actix_web::{error::ErrorUnauthorized, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;

type StdResult<T, V> = std::result::Result<T, V>;
type AtomicDb = Arc<Mutex<Database>>;
type AtomicFs = Arc<Mutex<FileLogger>>;
type AtomicRecv = Arc<Mutex<Receiver<bool>>>;

struct PipelineInfo {
    pool: web::Data<PipelinePool>,
    id: String,
    name: String,
    ex: AtomicDb,
    lg: AtomicFs,
    cm: Option<AtomicRecv>,
    vars: Arc<HashMap<String, String>>,
}

impl PipelineInfo {
    pub fn spawn(self) {
        thread::spawn(move || {
            if let Ok(mut rt) = Runtime::new() {
                rt.block_on(async move {
                    if let Err(e) =
                        Runner::from_file(self.name, self.ex, self.lg, self.cm, self.vars)
                            .await
                            .await
                    {
                        let _ = term::print_error(&e.to_string());
                    }
                    {
                        let mut pool = self.pool.senders.lock().unwrap();
                        pool.remove(&self.id);
                    }
                });
            }
        });
    }
}

pub struct ExecutePipelineSocket {
    hb: Instant,
    user: User,
    config: web::Data<BldConfig>,
    exec: Option<AtomicDb>,
    scanner: Option<FileScanner>,
    pool: web::Data<PipelinePool>,
}

impl ExecutePipelineSocket {
    pub fn new(user: User, config: web::Data<BldConfig>, pool: web::Data<PipelinePool>) -> Self {
        Self {
            hb: Instant::now(),
            user,
            config,
            exec: None,
            scanner: None,
            pool,
        }
    }

    fn heartbeat(act: &Self, ctx: &mut <Self as Actor>::Context) {
        if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
            println!("Websocket heartbeat failed, disconnecting!");
            ctx.stop();
            return;
        }
        ctx.ping(b"");
    }

    fn scan(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(scanner) = act.scanner.as_mut() {
            let content = scanner.fetch();
            for line in content.iter() {
                ctx.text(line);
            }
        }
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(exec) = act.exec.as_mut() {
            let exec = exec.lock().unwrap();
            if let Some(pipeline) = &exec.pipeline {
                if !pipeline.running {
                    ctx.stop();
                }
            }
        }
    }

    fn get_info(&mut self, data: &str) -> Result<PipelineInfo> {
        let info = serde_json::from_str::<ExecInfo>(data)?;
        let path = Pipeline::get_path(&info.name)?;
        if !path.is_file() {
            let message = String::from("pipeline file not found");
            return Err(BldError::IoError(message));
        }

        let id = Uuid::new_v4().to_string();
        let config = self.config.get_ref();
        let logs = path![&config.local.logs, format!("{}-{}", &info.name, id)]
            .display()
            .to_string();

        let mut db = Database::connect(&config.local.db)?;
        db.add(&id, &info.name, &self.user.name)?;

        let ex = Arc::new(Mutex::new(db));
        let (tx, rx) = mpsc::channel::<bool>();
        let rx = Arc::new(Mutex::new(rx));
        {
            let mut pool = self.pool.senders.lock().unwrap();
            pool.insert(id.clone(), tx);
        }

        let info = PipelineInfo {
            pool: self.pool.clone(),
            id,
            name: info.name,
            ex,
            lg: Arc::new(Mutex::new(FileLogger::new(&logs)?)),
            cm: Some(rx),
            vars: match info.variables {
                Some(vars) => Arc::new(vars),
                None => Arc::new(HashMap::<String, String>::new()),
            },
        };

        self.exec = Some(info.ex.clone());
        self.scanner = Some(FileScanner::new(&logs)?);

        Ok(info)
    }
}

impl Actor for ExecutePipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            ExecutePipelineSocket::heartbeat(act, ctx);
            ExecutePipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(10), |act, ctx| {
            ExecutePipelineSocket::scan(act, ctx);
            ExecutePipelineSocket::exec(act, ctx);
        });
    }
}

impl StreamHandler<StdResult<ws::Message, ws::ProtocolError>> for ExecutePipelineSocket {
    fn handle(&mut self, msg: StdResult<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                match self.get_info(&txt) {
                    Ok(pipeline_info) => {
                        pipeline_info.spawn();
                    }
                    Err(e) => {
                        eprintln!("{}", e.to_string());
                        ctx.text("Unable to run pipeline");
                        ctx.stop();
                    }
                };
            }
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_exec(
    user: Option<User>,
    req: HttpRequest,
    stream: web::Payload,
    config: web::Data<BldConfig>,
    pool: web::Data<PipelinePool>,
) -> StdResult<HttpResponse, Error> {
    let user = user.ok_or_else(|| ErrorUnauthorized(""))?;
    println!("{:?}", req);
    let res = ws::start(ExecutePipelineSocket::new(user, config, pool), &req, stream);
    println!("{:?}", res);
    res
}
