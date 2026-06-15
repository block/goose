use super::completion::GooseCompleter;
use super::input::{self, InputResult};
use super::HistoryManager;
use rustyline::ExternalPrinter;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Commands sent TO the input thread from the async event loop.
#[derive(Debug)]
pub enum InputCommand {
    /// Shut down the input thread.
    Shutdown,
    /// Resume prompting (after async processing is complete).
    Resume,
}

/// Events sent FROM the input thread to the async event loop.
#[derive(Debug)]
pub enum InputEvent {
    /// User submitted a line of input (processed into InputResult).
    Input(InputResult),
    /// The input thread has exited.
    Closed,
}

/// Shared state for interrupt coordination between the async loop
/// and the ConditionalEventHandler inside readline.
#[allow(dead_code)]
pub struct InterruptState {
    /// When true, the next keypress should save the buffer and interrupt readline.
    pub interrupt_requested: AtomicBool,
    /// The saved line buffer captured by the handler before interrupting.
    pub saved_line: Mutex<String>,
}

impl InterruptState {
    pub fn new() -> Self {
        Self {
            interrupt_requested: AtomicBool::new(false),
            saved_line: Mutex::new(String::new()),
        }
    }
}

/// Handle to communicate with the input thread and print above the prompt.
pub struct InputHandle {
    /// Send commands to the input thread (std channel — works from async context).
    pub command_tx: std::sync::mpsc::Sender<InputCommand>,
    /// Receive events from the input thread (tokio channel — works in select!).
    pub event_rx: mpsc::UnboundedReceiver<InputEvent>,
    /// Print messages above the prompt without disturbing user input.
    pub printer: Printer,
    /// Shared interrupt state (for future forced-turn approval mechanism).
    #[allow(dead_code)]
    pub interrupt_state: Arc<InterruptState>,
    /// Join handle for the input thread.
    join_handle: Option<std::thread::JoinHandle<()>>,
}

/// Wrapper around rustyline's ExternalPrinter that is Send-safe.
pub struct Printer {
    inner: Box<dyn ExternalPrinter + Send>,
}

impl Printer {
    /// Print a message above the prompt without disturbing user input.
    pub fn print(&mut self, msg: String) {
        let _ = self.inner.print(msg);
    }
}

impl InputHandle {
    /// Send a shutdown command and wait for the thread to exit.
    pub fn shutdown(&mut self) {
        let _ = self.command_tx.send(InputCommand::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for InputHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Spawn the input thread. Returns an InputHandle for the async loop to use.
///
/// The input thread owns the rustyline Editor and processes readline in a loop.
/// The ExternalPrinter is created BEFORE the editor moves to the thread —
/// it stays on the async side for non-blocking output above the prompt.
pub fn spawn_input_thread(
    mut editor: rustyline::Editor<GooseCompleter, rustyline::history::DefaultHistory>,
) -> InputHandle {
    let (command_tx, command_rx) = std::sync::mpsc::channel::<InputCommand>();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<InputEvent>();
    let interrupt_state = Arc::new(InterruptState::new());

    // Create ExternalPrinter BEFORE moving editor to the thread.
    // This gives us a handle to print above the prompt from the async side.
    let printer = editor
        .create_external_printer()
        .expect("Failed to create ExternalPrinter — terminal may not be a TTY");

    let handle = std::thread::Builder::new()
        .name("goose-input".into())
        .spawn(move || {
            input_thread_main(editor, command_rx, event_tx);
        })
        .expect("Failed to spawn input thread");

    InputHandle {
        command_tx,
        event_rx,
        printer: Printer {
            inner: Box::new(printer),
        },
        interrupt_state,
        join_handle: Some(handle),
    }
}

/// Main loop of the input thread.
fn input_thread_main(
    mut editor: rustyline::Editor<GooseCompleter, rustyline::history::DefaultHistory>,
    command_rx: std::sync::mpsc::Receiver<InputCommand>,
    event_tx: mpsc::UnboundedSender<InputEvent>,
) {
    let history_manager = HistoryManager::new();
    history_manager.load(&mut editor);

    loop {
        // Check for pending commands before starting readline
        match command_rx.try_recv() {
            Ok(InputCommand::Shutdown) => break,
            Ok(InputCommand::Resume) => {} // stale Resume, ignore
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }

        // Run readline (blocks until user submits or interrupts)
        let result = input::get_input(&mut editor, None);

        match result {
            Ok(input_result) => {
                let is_exit = matches!(input_result, InputResult::Exit);
                let is_retry = matches!(input_result, InputResult::Retry);
                history_manager.save(&mut editor);
                if event_tx.send(InputEvent::Input(input_result)).is_err() {
                    break;
                }
                if is_exit {
                    break;
                }
                // After sending a non-trivial event, wait for the async side
                // to signal Resume before re-prompting.
                if !is_retry {
                    match command_rx.recv() {
                        Ok(InputCommand::Resume) => {}
                        Ok(InputCommand::Shutdown) | Err(_) => break,
                    }
                }
            }
            Err(_) => {
                let _ = event_tx.send(InputEvent::Closed);
                break;
            }
        }
    }

    let _ = event_tx.send(InputEvent::Closed);
}
