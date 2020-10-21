use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

pub struct Worker {
    reader: Lines<BufReader<ChildStdout>>,
    writer: ChildStdin,
}

impl Worker {
    pub fn new() -> Result<Self> {
        let mut child = Command::new("php")
            .arg("worker.php")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("could not get stdout"))?;

        let writer = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("could not get stdin"))?;
        let reader = BufReader::new(stdout).lines();

        tokio::spawn(async move {
            let status = child.await.expect("child process encountered an error");
            println!("child status: {}", status);
        });

        Ok(Self { reader, writer })
    }

    pub async fn exec(&mut self, payload: &str) -> Result<String> {
        self.writer.write(payload.as_bytes()).await?;
        self.writer.write("\n".as_bytes()).await?;

        match timeout(Duration::from_millis(100), self.reader.next_line()).await {
            Ok(Ok(Some(output))) => Ok(output),
            Ok(_) => Err(anyhow!("could not read output from worker")),
            Err(_) => Err(anyhow!("worker timeout")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn communicating_with_worker() -> Result<()> {
        let mut worker = Worker::new()?;

        assert_eq!(
            worker.exec(r#"{"name":"world"}"#).await?,
            r#"{"message":"hello world"}"#.to_string(),
        );

        Ok(())
    }
}
