use crate::helper::DynError;
use nix::{
    libc,
    sys::signal::{killpg, signal, SigHandler, Signal},
    unistd::{tcgetpgrp, tcsetpgrp, Pid},
};
use rustyline::{error::ReadlineError, Editor};
use signal_hook::{consts::*, iterator::Signals};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    process::exit,
    sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
    thread,
};

/// システムコールのラッパ。`EINTR`=システムコールが割り込みによって失敗したときリトライする
fn syscall<F, T>(f: F) -> Result<T, nix::Error>
where
    F: Fn() -> Result<T, nix::Error>,
{
    loop {
        match f() {
            Err(nix::Error::EINTR) => (),
            res => return res,
        }
    }
}

/// workerスレッドが受信するメッセージ
enum WorkerMsg {
    /// 受信したシグナル
    Signal(i32),
    /// 受信したコマンド
    Cmd(String),
}

/// mainスレッドが受信するメッセージ
enum ShellMsg {
    /// シェルの読み込み再開。値は最後の終了コード
    Continue(i32),
    /// シェルを終了。値は終了コード
    Quit(i32),
}

#[derive(Debug)]
pub struct Shell {
    logfile: String,
}

impl Shell {
    pub fn new(logfile: &str) -> Self {
        Self {
            logfile: logfile.to_string(),
        }
    }

    pub fn run(&self) -> Result<(), DynError> {
        unsafe { signal(Signal::SIGTTOU, SigHandler::SigIgn).unwrap() };
        let mut rl = Editor::<()>::new()?;
        if let Err(e) = rl.load_history(&self.logfile) {
            eprintln!("ZeroSh: ヒストリファイルの読み込みに失敗: {e}")
        }

        let (worker_tx, worker_rx) = channel();
        let (shell_tx, shell_rx) = sync_channel(0);

        spawn_sig_handler(worker_tx.clone())?;
        Worker::new().spawn(worker_rx, shell_tx);

        let exit_val;
        let mut prev = 0;
        loop {
            let face = if prev == 0 { '\u{1F642}' } else { '\u{1F480}' };
            match rl.readline(&format!("ZeroSh {face} %> ")) {
                Ok(line) => {
                    let line_trimed = line.trim();
                    if line_trimed.is_empty() {
                        continue;
                    } else {
                        rl.add_history_entry(line_trimed);
                    }

                    worker_tx.send(WorkerMsg::Cmd(line)).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Continue(n) => prev = n,
                        ShellMsg::Quit(n) => {
                            exit_val = n;
                            break;
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => eprintln!("ZeroSh: 終了はCtrl+d"),
                Err(ReadlineError::Eof) => {
                    worker_tx.send(WorkerMsg::Cmd("exit".to_string())).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Quit(n) => {
                            exit_val = n;
                            break;
                        }
                        _ => {
                            panic!("exitに失敗")
                        }
                    }
                }
                Err(e) => {
                    eprintln!("ZeroSh: 読み込みエラー\n{e}");
                    exit_val = 1;
                    break;
                }
            }
        }

        if let Err(e) = rl.save_history(&self.logfile) {
            eprintln!("ZeroSh: ヒストリファイルへの書き込みに失敗: {e}");
        }

        exit(exit_val)
    }
}

/// signal_handlerのスレッド
fn spawn_sig_handler(tx: Sender<WorkerMsg>) -> Result<(), DynError> {
    // `SIGINT`,`SIGTSTP` => Ctrl+c, Ctrl+z用
    // `SIGCHLD`=>子プロセスの状態変化検知用
    let mut signals = Signals::new([SIGINT, SIGTSTP, SIGCHLD])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            tx.send(WorkerMsg::Signal(sig)).unwrap();
        }
    });
    Ok(())
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ProcState {
    /// 実行中
    Run,
    /// 停止中
    Stop,
}

#[derive(Debug, Clone)]
struct ProcInfo {
    /// プロセスの実行状態
    state: ProcState,
    /// プロセスグループid
    pgid: Pid,
}

#[derive(Debug)]
struct Worker {
    /// 終了コード
    exit_val: i32,
    /// 実行中のプロセスグループid
    fg: Option<Pid>,
    /// ジョブidと(プロセスグループid,実行コマンド)のマップ
    jobs: BTreeMap<usize, (Pid, String)>,
    /// プロセスグループidに属するプロセスidへのマップ
    pgid_to_pids: HashMap<usize, HashSet<Pid>>,
    /// プロセスidからプロセスグループidへのマップ
    pid_to_info: HashMap<Pid, ProcInfo>,
    /// `Shell`のプロセスグループid
    shell_pgid: Pid,
}

type CmdResult<'a> = Result<Vec<(&'a str, Vec<&'a str>)>, DynError>;

fn parse_cmd(line: &str) -> CmdResult<'_> {
    let cmds = line.split('|').collect::<Vec<&str>>();
    let mut res = vec![];

    for cmd in cmds {
        // 両端の空白をまず除去する
        let cmd = cmd.trim();
        // 空白のみの場合は無視する
        if cmd.is_empty() {
            continue;
        }

        let mut cmd_trimmed = cmd.split(' ').map(|s| s.trim());
        // cmdはemptyではないので、少なくとも１回はunwrapできる
        let first = cmd_trimmed.next().unwrap();

        // 残りはVecにまとめる
        let rest = cmd_trimmed.collect::<Vec<_>>();

        res.push((first, rest));
    }

    if res.is_empty() {
        Err("invalid command".into())
    } else {
        Ok(res)
    }
}

impl Worker {
    fn new() -> Self {
        Worker {
            exit_val: 0,
            fg: None,
            jobs: Default::default(),
            pgid_to_pids: Default::default(),
            pid_to_info: Default::default(),
            shell_pgid: tcgetpgrp(libc::STDIN_FILENO).unwrap(),
        }
    }

    fn spawn(mut self, worker_rx: Receiver<WorkerMsg>, shell_tx: SyncSender<ShellMsg>) {
        thread::spawn(move || {
            for msg in worker_rx.iter() {
                match msg {
                    WorkerMsg::Cmd(line) => match parse_cmd(&line) {
                        Ok(cmd) => {
                            if self.build_in_cmd(&cmd, &shell_tx) {
                                continue;
                            }

                            todo!()
                        }
                        Err(e) => {
                            eprintln!("ZeroSh: {e}");
                            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap()
                        }
                    },
                    WorkerMsg::Signal(_) => {
                        todo!()
                    }
                }
            }
        });
    }

    fn build_in_cmd(&mut self, cmd: &[(&str, Vec<&str>)], shell_tx: &SyncSender<ShellMsg>) -> bool {
        if cmd.len() > 1 {
            return false;
        }

        match cmd[0].0 {
            "exit" => self.run_exit(&cmd[0].1, shell_tx),
            "jobs" => self.run_jobs(&cmd[0].1, shell_tx),
            "fg" => self.run_fg(&cmd[0].1, shell_tx),
            "cd" => self.run_cd(&cmd[0].1, shell_tx),
            _ => false,
        }
    }

    /// シェルを抜ける
    ///
    /// `exit exit_code`の形で終了コードを指定できる
    fn run_exit(&mut self, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
        // 何かを実行中の場合は終了しない
        if !self.jobs.is_empty() {
            eprintln!("ZeroSh: ジョブが実行中のため終了できません");
            self.exit_val = 1;
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
            return true;
        };

        let exit_val = if let Some(s) = args.get(1) {
            if let Ok(n) = s.parse::<i32>() {
                n
            } else {
                // `exit XXX`の終了コードが整数でない
                eprintln!("ZeroSh: {s}は不正な引数です");
                self.exit_val = 1;
                shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                return true;
            }
        } else {
            self.exit_val
        };

        shell_tx.send(ShellMsg::Quit(exit_val)).unwrap();
        true
    }

    /// 現在実行中のジョブを一覧表示する
    fn run_jobs(&mut self, _args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
        for (pgid, cmd) in self.jobs.values() {
            println!("[{pgid}] \t{cmd}");
        }

        self.exit_val = 0;
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
        true
    }

    /// 指定されたコマンドをバックグラウンド実行からフォアグラウンド実行に切り替える
    ///
    /// `fg cmd_id`という形で指定する
    fn run_fg(&mut self, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
        self.exit_val = 1; // ひとまず失敗にしておく

        if args.len() < 2 {
            eprintln!("usage: fg 数字");
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
            return true;
        }

        if let Ok(n) = args[1].parse::<usize>() {
            if let Some((pgid, cmd)) = self.jobs.get(&n) {
                eprintln!("[{n}] 再開 \t{cmd}");

                self.fg = Some(*pgid);
                tcsetpgrp(libc::STDIN_FILENO, *pgid).unwrap();

                killpg(*pgid, Signal::SIGCONT).unwrap();
                return true;
            }
        };
        eprintln!("{}というジョブは見つかりませんでした", args[1]);
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
        true
    }

    /// カレントディレクトリを移動する
    fn run_cd(&mut self, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
        self.exit_val = 1;
        if args.len() < 2 {
            eprintln!("usage: cd 移動先");
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
            return true;
        }
        std::env::set_current_dir(args[0]).unwrap();
        self.exit_val = 0;
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_parse_cmd() {
        let cmd = "echo hello | less";

        assert_eq!(
            parse_cmd(cmd).unwrap(),
            vec![("echo", vec!["hello"]), ("less", vec![])]
        );
    }

    #[test]
    fn empty_parse_cmd() {
        let cmd = "";

        assert!(parse_cmd(cmd).is_err());
    }

    #[test]
    fn empty_pipe_parse_cmd() {
        let cmd = "echo hello | | less";

        assert_eq!(
            parse_cmd(cmd).unwrap(),
            vec![("echo", vec!["hello"]), ("less", vec![])]
        );
    }
}
