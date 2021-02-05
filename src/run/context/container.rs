use crate::config::BldConfig;
use crate::persist::Logger;
use crate::types::{BldError, CheckStopSignal, Result};
use futures_util::StreamExt;
use shiplift::tty::TtyChunk;
use shiplift::{ContainerOptions, Docker, ExecContainerOptions, ImageListOptions, PullOptions};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

type AtomicRecv = Arc<Mutex<Receiver<bool>>>;

pub struct Container {
    pub config: Option<Rc<BldConfig>>,
    pub img: String,
    pub client: Option<Docker>,
    pub id: Option<String>,
    pub lg: Arc<Mutex<dyn Logger>>,
}

impl Container {
    fn get_client(&self) -> Result<&Docker> {
        match &self.client {
            Some(client) => Ok(client),
            None => Err(BldError::Other("container not started".to_string())),
        }
    }

    fn get_id(&self) -> Result<&str> {
        match &self.id {
            Some(id) => Ok(id),
            None => Err(BldError::Other("container id not found".to_string())),
        }
    }

    fn docker(config: &Rc<BldConfig>) -> Result<Docker> {
        let url = config.local.docker_url.parse()?;
        let host = Docker::host(url);
        Ok(host)
    }

    async fn pull(client: &Docker, image: &str, logger: &mut Arc<Mutex<dyn Logger>>) -> Result<()> {
        let options = ImageListOptions::builder().filter_name(image).build();
        let images = client.images().list(&options).await?;
        if images.is_empty() {
            {
                let mut logger = logger.lock().unwrap();
                logger.info(&format!("Download image: {}", image));
            }
            let options = PullOptions::builder().image(image).build();
            let mut pull_iter = client.images().pull(&options);
            while let Some(progress) = pull_iter.next().await {
                let info = progress?;
                {
                    let mut logger = logger.lock().unwrap();
                    logger.dumpln(&info.to_string());
                }
                sleep(Duration::from_millis(100));
            }
        }
        Ok(())
    }

    async fn create(
        client: &Docker,
        image: &str,
        logger: &mut Arc<Mutex<dyn Logger>>,
    ) -> Result<String> {
        Container::pull(client, image, logger).await?;
        let options = ContainerOptions::builder(&image).tty(true).build();
        let info = client.containers().create(&options).await?;
        client.containers().get(&info.id).start().await?;
        Ok(info.id)
    }

    pub fn new(img: &str, lg: Arc<Mutex<dyn Logger>>) -> Self {
        Self {
            config: None,
            img: img.to_string(),
            client: None,
            id: None,
            lg,
        }
    }

    pub async fn start(&self, config: Rc<BldConfig>) -> Result<Self> {
        let mut lg = self.lg.clone();
        let client = Container::docker(&config)?;
        let id = Container::create(&client, &self.img, &mut lg).await?;
        Ok(Self {
            config: Some(config),
            img: self.img.to_string(),
            client: Some(client),
            id: Some(id),
            lg: self.lg.clone(),
        })
    }

    pub async fn sh(
        &self,
        working_dir: &Option<String>,
        input: &str,
        cm: &Option<AtomicRecv>,
    ) -> Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;
        let input = match working_dir {
            Some(wd) => format!("cd {} && {}", &wd, input),
            None => input.to_string(),
        };
        let cmd = vec!["bash", "-c", &input];
        let options = ExecContainerOptions::builder()
            .cmd(cmd)
            .attach_stdout(true)
            .attach_stderr(true)
            .build();
        let container = client.containers().get(&id);
        let mut exec_iter = container.exec(&options);
        while let Some(result) = exec_iter.next().await {
            cm.check_stop_signal()?;
            let chunk = match result {
                Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => return Err(BldError::ShipliftError(e.to_string())),
            };
            {
                let mut logger = self.lg.lock().unwrap();
                logger.dump(&chunk);
            }
            sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub async fn dispose(&self) -> Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;
        client.containers().get(id).stop(None).await?;
        client.containers().get(id).delete().await?;
        Ok(())
    }
}
