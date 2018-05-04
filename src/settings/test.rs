use std::{
  collections::{HashMap, HashSet},
  env,
  error::Error,
  sync::{LockResult, Mutex, MutexGuard},
};

use super::*;

lazy_static! {
  // HACK: mutex to prevent tests clobbering each other's environment variables
  static ref ENVIRONMENT_MUTEX: Mutex<bool> = Mutex::new(true);
}

struct CleanEnvironment<'e>
{
  vars_to_reinstate: HashMap<String, String>,
  keys_to_clear: HashSet<String>,
  _lock: LockResult<MutexGuard<'e, bool>>,
}

impl<'e> CleanEnvironment<'e>
{
  pub fn new(keys: Vec<&str>) -> CleanEnvironment
  {
    let mut snapshot = CleanEnvironment {
      vars_to_reinstate: HashMap::new(),
      keys_to_clear: HashSet::new(),
      _lock: ENVIRONMENT_MUTEX.lock(),
    };

    snapshot.initialise(keys);

    snapshot
  }

  fn initialise(&mut self, keys: Vec<&str>)
  {
    for key in keys {
      if let Ok(value) = env::var(key) {
        self.vars_to_reinstate.insert(String::from(key), value);
        env::remove_var(key);
      } else {
        self.keys_to_clear.insert(String::from(key));
      }
    }
  }
}

impl<'e> Drop for CleanEnvironment<'e>
{
  fn drop(&mut self)
  {
    for (key, value) in &self.vars_to_reinstate {
      env::set_var(key, &value);
    }

    for key in &self.keys_to_clear {
      env::remove_var(key);
    }
  }
}

#[test]
fn env_vars_take_precedence()
{
  let _clean_env = CleanEnvironment::new(vec![
    "FXA_EMAIL_SENDER",
    "FXA_EMAIL_SMTP_HOST",
    "FXA_EMAIL_SMTP_PORT",
    "FXA_EMAIL_SMTP_USER",
    "FXA_EMAIL_SMTP_PASSWORD",
  ]);

  match Settings::new() {
    Ok(settings) => {
      let sender = format!("{}{}", "1", &settings.sender);
      let host = format!("{}{}", &settings.smtp.host, "2");
      let port = settings.smtp.port + 3;
      let user = if let Some(ref user) = settings.smtp.user {
        format!("{}{}", user, "4")
      } else {
        String::from("4")
      };
      let password = if let Some(ref password) = settings.smtp.password {
        format!("{}{}", password, "5")
      } else {
        String::from("5")
      };

      env::set_var("FXA_EMAIL_SENDER", &sender);
      env::set_var("FXA_EMAIL_SMTP_HOST", &host);
      env::set_var("FXA_EMAIL_SMTP_PORT", &port.to_string());
      env::set_var("FXA_EMAIL_SMTP_USER", &user);
      env::set_var("FXA_EMAIL_SMTP_PASSWORD", &password);

      match Settings::new() {
        Ok(env_settings) => {
          assert_eq!(env_settings.sender, sender);
          assert_eq!(env_settings.smtp.host, host);
          assert_eq!(env_settings.smtp.port, port);

          if let Some(env_user) = env_settings.smtp.user {
            assert_eq!(env_user, user);
          } else {
            assert!(false, "smtp.user was not set");
          }

          if let Some(env_password) = env_settings.smtp.password {
            assert_eq!(env_password, password);
          } else {
            assert!(false, "smtp.password was not set");
          }
        }
        Err(error) => {
          println!("{}", error);
          assert!(false);
        }
      }
    }
    Err(error) => {
      println!("{}", error);
      assert!(false);
    }
  }
}

#[test]
fn invalid_host()
{
  let _clean_env = CleanEnvironment::new(vec!["FXA_EMAIL_SMTP_HOST"]);
  env::set_var("FXA_EMAIL_SMTP_HOST", "https://mail.google.com/");

  match Settings::new() {
    Ok(_settings) => assert!(false, "Settings::new should have failed"),
    Err(error) => assert_eq!(error.description(), "configuration error"),
  }
}

#[test]
fn invalid_sender()
{
  let _clean_env = CleanEnvironment::new(vec!["FXA_EMAIL_SENDER"]);
  env::set_var("FXA_EMAIL_SENDER", "wibble");

  match Settings::new() {
    Ok(_settings) => assert!(false, "Settings::new should have failed"),
    Err(error) => assert_eq!(error.description(), "configuration error"),
  }
}
