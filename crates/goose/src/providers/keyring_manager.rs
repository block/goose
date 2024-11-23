use std::env;
use std::io::{self, Write};
use keyring;

pub struct KeyringManager {
    service_name: String,
    key_name: String,
}

impl KeyringManager {
    pub fn new(service: &str, key: &str) -> Self {
        Self {
            service_name: service.to_string(),
            key_name: key.to_string(),
        }
    }

    /// Retrieves the API key, checks the keyring and environment, and prompts
    /// the user if necessary, with an option to store the key in the keyring.
    pub fn retrieve_api_key(&self, provider_name: &str) -> Option<String> {
        if let Ok(kr) = keyring::Entry::new(&self.service_name, &self.key_name) {
            if let Ok(api_key) = kr.get_password() {
                println!("{} API key found in keyring", provider_name);
                env::set_var(&self.key_name, &api_key);
                return Some(api_key);
            } else if let Ok(api_key) = env::var(&self.key_name) {
                println!("{} API key found in environment.", provider_name);
                self.prompt_to_save_key(&kr, &api_key);
                return Some(api_key);
            } else {
                return self.prompt_for_key(&kr, provider_name);
            }
        }
        None
    }

    fn prompt_for_key(&self, kr: &keyring::Entry, provider_name: &str) -> Option<String> {
        print!("Please enter your {} API key: ", provider_name);
        io::stdout().flush().ok();
        let mut api_key = String::new();
        if io::stdin().read_line(&mut api_key).is_ok() {
            let api_key = api_key.trim().to_string();
            env::set_var(&self.key_name, &api_key);
            self.prompt_to_save_key(kr, &api_key);
            return Some(api_key);
        }
        None
    }

    fn prompt_to_save_key(&self, kr: &keyring::Entry, api_key: &str) {
        print!("Would you like to save this API key to the keyring for future sessions? (y/n): ");
        io::stdout().flush().ok();
        let mut save = String::new();
        if io::stdin().read_line(&mut save).is_ok() {
            if save.trim().to_lowercase() == "y" {
                if kr.set_password(api_key).is_ok() {
                    println!("API key saved to keyring successfully!");
                } else {
                    println!("Failed to save API key to keyring");
                }
            }
        }
    }
}
