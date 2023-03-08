use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use vampirc_uci::UciMessage;

#[derive(Debug)]
pub struct RawEngine {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pub stderr: Option<BufReader<ChildStderr>>,
}

impl RawEngine {
    pub async fn new(path: &Path, args: &[String]) -> tokio::io::Result<Self> {
        let mut child = Command::new(path)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(args)
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = Some(BufReader::new(child.stderr.take().unwrap()));

        Ok(Self {
            _child: child,
            stdin,
            stdout,
            stderr,
        })
    }

    pub async fn send(&mut self, message: UciMessage) -> tokio::io::Result<()> {
        let message = format!("{}\n", message);
        self.stdin.write_all(message.as_bytes()).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> tokio::io::Result<Option<UciMessage>> {
        let mut message = String::new();
        if self.stdout.read_line(&mut message).await? == 0 {
            return Ok(None);
        }
        let message = vampirc_uci::parse_one(&message);
        Ok(Some(message))
    }
}
