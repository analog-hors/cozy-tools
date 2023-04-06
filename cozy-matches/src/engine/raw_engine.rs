use std::path::Path;
use std::process::Stdio;

use cozy_uci::command::UciCommand;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use cozy_uci::UciFormatOptions;
use cozy_uci::remark::UciRemark;

use super::error::EngineError;

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

    pub async fn send(&mut self, cmd: &UciCommand, options: &UciFormatOptions) -> Result<(), EngineError> {
        self.stdin.write_all(cmd.format(options).as_bytes()).await?;
        Ok(())
    }

    pub async fn recv(&mut self, options: &UciFormatOptions) -> Result<Option<UciRemark>, EngineError> {
        let mut rmk = String::new();
        if self.stdout.read_line(&mut rmk).await? == 0 {
            return Ok(None);
        }
        let rmk = UciRemark::parse_from(&rmk, options)
            .map_err(|e| EngineError::InvalidMessage(rmk, e))?;
        Ok(Some(rmk))
    }
}
