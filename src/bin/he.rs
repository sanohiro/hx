use std::io::{self, IsTerminal, Read, Write as _};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste, EnableFocusChange, DisableFocusChange},
    execute, queue,
    terminal::{
        disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};

use he::app::App;

/// Terminal hex editor inspired by Stirling
#[derive(Parser, Debug)]
#[command(name = "hx")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to open
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// Bytes per row (default: 16)
    #[arg(short, long, default_value = "16")]
    bytes_per_row: usize,

    /// Read-only mode
    #[arg(short, long)]
    readonly: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 標準入力からデータを読み込む（パイプされている場合）
    let stdin_data = if !io::stdin().is_terminal() {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Some(data)
    } else {
        None
    };

    // ターミナルの初期化
    // マウスモードは無効（ターミナルでのテキスト選択・コピーを優先）
    // Alternate Screenでトラックパッドスクロールによるバッファ移動を防止
    // Bracketed Pasteでペースト内容を一括取り込み
    // Focus Eventsでフォーカス変更を検出
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableBracketedPaste,
        EnableFocusChange
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // アプリケーションの実行
    let result = run_app(&mut terminal, args, stdin_data);

    // ターミナルの後処理
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableFocusChange,
        DisableBracketedPaste,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, args: Args, stdin_data: Option<Vec<u8>>) -> Result<()> {
    let mut app = App::new();

    // データを読み込む（優先順位: ファイル > 標準入力）
    if let Some(ref path) = args.file {
        app.open(path)?;
    } else if let Some(data) = stdin_data {
        app.load_bytes(data);
    }

    // ウィンドウタイトルを設定
    update_title(terminal.backend_mut(), &app)?;

    // メインループ
    loop {
        // Synchronized Update: 描画のちらつきを防止
        queue!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
        terminal.draw(|f| app.draw(f))?;
        queue!(terminal.backend_mut(), EndSynchronizedUpdate)?;
        terminal.backend_mut().flush()?;

        app.handle_event()?;

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

/// ウィンドウタイトルを更新
fn update_title(backend: &mut CrosstermBackend<io::Stdout>, app: &App) -> Result<()> {
    let title = format!(
        "hx - {}{}",
        app.filename().unwrap_or("[New File]"),
        if app.is_modified() { " [+]" } else { "" }
    );
    execute!(backend, SetTitle(&title))?;
    Ok(())
}
