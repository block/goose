mod macos;
mod windows;

#[cfg(target_os = "windows")]
pub use self::windows::WindowsAutomation;

#[cfg(target_os = "macos")]
pub use self::macos::MacOSAutomation;

pub trait SystemAutomation: Send + Sync {
    fn execute_system_script(&self, script: &str) -> std::io::Result<String>;
    fn get_shell_command(&self) -> (&'static str, &'static str); // (shell, arg)
    fn get_temp_path(&self) -> std::path::PathBuf;
}

pub fn create_system_automation() -> Box<dyn SystemAutomation + Send + Sync> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsAutomation)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSAutomation)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        unimplemented!("Unsupported operating system")
    }
}
