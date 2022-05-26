/*
 Prod
 Copyright 2021-2022 Peter Pearson.
 Licensed under the Apache License, Version 2.0 (the "License");
 You may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 http://www.apache.org/licenses/LICENSE-2.0
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 ---------
*/

#![allow(dead_code)]

extern crate rpassword;
use rpassword::read_password;

use crate::control::control_actions::{ActionResult, ControlActionType};
use crate::control::control_common::{ControlSession, ControlSessionParams, ControlSessionUserAuth, UserAuthUserPass};

use super::control_actions::{ControlActions, ActionProvider};

use super::action_provider_linux_debian;
use super::action_provider_linux_fedora;

pub struct ControlManager {
    
}

#[derive(Clone, Debug)]
pub enum CommandResult {
    ErrorCantConnect(String),
    ErrorAuthenticationIssue(String),
    Failed(String),
    CommandRunOkay(String),
}

impl ControlManager {
    pub fn new() -> ControlManager {
        let manager = ControlManager { };
        return manager;
    }

    // Not really happy with this, but I can't work out how to nicely self-configure/inspect in a registry,
    // so this seems next best thing...
    fn create_provider(&self, provider: &str, session_params: ControlSessionParams) -> Option<Box<dyn ActionProvider>> {
        if provider == action_provider_linux_debian::AProviderLinuxDebian::name() {
            return Some(Box::new(action_provider_linux_debian::AProviderLinuxDebian::new(session_params)));
        }
        else if provider == action_provider_linux_fedora::AProviderLinuxFedora::name() {
            return Some(Box::new(action_provider_linux_fedora::AProviderLinuxFedora::new(session_params)));
        }

        return None;
    }

    pub fn run_command(&self, host: &str, command: &str) -> CommandResult {
        println!("Connecting to host: {}...", host);

        let host_target = format!("{}:22", host);

        println!("Enter password:");
        let password = read_password().unwrap();

        let username = "peter";

        let control_session_user_auth = ControlSessionUserAuth::UserPass(UserAuthUserPass::new(&username, &password));

        let session_params = ControlSessionParams::new(&host_target, control_session_user_auth, true);

#[cfg(feature = "ssh")]
        let connection = ControlSession::new_ssh(session_params);

#[cfg(not(feature = "ssh"))]
        let connection = ControlSession::new_dummy_debug(session_params);

        if connection.is_none() {
            eprintln!("Error connecting to hostname...");
            return CommandResult::ErrorCantConnect("".to_string());
        }
        let mut connection = connection.unwrap();

        connection.conn.send_command(command);

        return CommandResult::CommandRunOkay(connection.conn.get_previous_stdout_response().to_string());
    }

    pub fn perform_actions(&self, actions: &ControlActions) {
        if actions.actions.is_empty() {
            eprintln!("Error: no actions specified.");
        }

        // TODO: come up with a better way of handling this partial initialisation / ordering dilema to work
        //       out if a provider exists before querying for usernames and passwords...
        let mut session_params = ControlSessionParams::new("", actions.auth.clone(), true);

        // temp creation to check it exists as a provider name...
        let provider = self.create_provider(&actions.provider, session_params);
        if provider.is_none() {
            eprintln!("Error: Can't find control provider: '{}'.", actions.provider);
            return;
        }

        let provider = provider.unwrap();

        let mut asked_for_hostname = false;
//        let mut asked_for_username = false;

        let mut hostname = String::new();
        if actions.host.is_empty() || actions.host == "$PROMPT" {
            eprintln!("Please enter hostname to connect to:");
            std::io::stdin().read_line(&mut hostname).expect("Error reading hostname from std input");
            hostname = hostname.trim().to_string();

            asked_for_hostname = true;
        }
        else {
            hostname = actions.host.clone();
        }

        // connect to host
        let host_target = hostname;

        // we take a local copy, so we can modify it and pass it in to be used in a final state...
        let mut auth = actions.auth.clone();

        // attempt to generalise the 'user' part for both enums, as it's needed for both,
        // but it makes things a bit verbose...
        let config_username = 
            match &auth {
                ControlSessionUserAuth::UserPass(userpass) => &userpass.username,
                ControlSessionUserAuth::PublicKey(publickey) => &publickey.username,
            }.clone();

        let mut username = String::new();
        if config_username.is_empty() || config_username == "$PROMPT" {
            eprintln!("Please enter username to authenticate with:");
            std::io::stdin().read_line(&mut username).expect("Error reading username from std input");
            username = username.trim().to_string();

//            asked_for_username = true;
        }
        else {
            username = config_username;
        }

        // now do the two enum types separately, and apply the above username to the contents of that
        // enum...
        if let ControlSessionUserAuth::UserPass(userpass) = &mut auth {
            userpass.username = username.clone();

            // TODO: do we want to maybe allow empty passwords?
            if userpass.password.is_empty() || userpass.password == "$PROMPT" {
                if !asked_for_hostname {
                    eprintln!("Enter password for user '{}' on host '{}':", &username, &host_target);
                }
                else {
                    eprintln!("Enter password:");
                }
                userpass.password = read_password().unwrap();
            }
            
        }
        else if let ControlSessionUserAuth::PublicKey(publickey) = &mut auth {
            publickey.username = username.clone();

            // explicitly allow empty passphrases for now...
            if publickey.passphrase == "$PROMPT" {
                if !asked_for_hostname {
                    eprintln!("Enter key passphrase for user '{}' on host '{}':", &username, &host_target);
                }
                else {
                    eprintln!("Enter key passphrase:");
                }
                publickey.passphrase = read_password().unwrap();
            }
        }

        // Now configure ControlSessionParams properly here...
        // TODO: as above, not really happy with this, but there's various "not great" ways of solving the issue
        //       I don't like, so I'm happier (just) with this for the moment...
        session_params = ControlSessionParams::new(&host_target, auth, true);

#[cfg(feature = "ssh")]
        let connection = ControlSession::new_ssh(session_params);

#[cfg(not(feature = "ssh"))]
        let connection = ControlSession::new_dummy_debug(session_params);

        if connection.is_none() {
            eprintln!("Error connecting to hostname...");
            return;
        }
        let mut connection = connection.unwrap();

/*
        let closure = || provider.add_user(&mut connection, &actions.actions[0]);
        let mut map : BTreeMap<ControlActionType, &dyn Fn(&mut ControlConnection, &ControlAction) -> ActionResult> = BTreeMap::new();
        map.insert(ControlActionType::AddUser, &closure as &dyn Fn(_, _) -> _);
*/

        let num_actions = actions.actions.len();
        eprintln!("Running {} {}...", num_actions, if num_actions == 1 {"action"} else {"actions"});

        let mut success = true;

        for (count, action) in actions.actions.iter().enumerate() {
            // TODO: Better (automatic - based off lookup) despatch than this...
            //       Although it's not clear how to easily do that (see above attempt), or if
            //       there's actually a benefit to doing it that way...

            let result = match action.action {
                ControlActionType::GenericCommand => {
                    provider.generic_command(&mut connection, action)
                },
                ControlActionType::AddUser => {
                    provider.add_user(&mut connection, action)
                },
                ControlActionType::CreateDirectory => {
                    provider.create_directory(&mut connection, action)
                },
                ControlActionType::InstallPackages => {
                    provider.install_packages(&mut connection, action)
                },
                ControlActionType::RemovePackages => {
                    provider.remove_packages(&mut connection, action)
                },
                ControlActionType::SystemCtl => {
                    provider.systemctrl(&mut connection, action)
                },
                ControlActionType::Firewall => {
                    provider.firewall(&mut connection, action)
                },
                ControlActionType::EditFile => {
                    provider.edit_file(&mut connection, action)
                },
                ControlActionType::CopyPath => {
                    provider.copy_path(&mut connection, action)
                },
                ControlActionType::DownloadFile => {
                    provider.download_file(&mut connection, action)
                },
                ControlActionType::TransmitFile => {
                    provider.transmit_file(&mut connection, action)
                },
                ControlActionType::ReceiveFile => {
                    provider.receive_file(&mut connection, action)
                },
                ControlActionType::CreateSymlink => {
                    provider.create_symlink(&mut connection, action)
                },
                ControlActionType::SetTimeZone => {
                    provider.set_time_zone(&mut connection, action)
                }
                ControlActionType::NotSet | ControlActionType::Unrecognised => {
                   ActionResult::Failed("Invalid Action Type".to_string())
                }
            };

            if result == ActionResult::NotImplemented {
                eprintln!("Error running action index {} : {} - the action provider does not implement this action...",
                            count, action.action);
                success = false;
                break;
            }

            if let ActionResult::InvalidParams(str) = result {
                eprintln!("Error running action index {} : {} - invalid parameters were provided for this action: {}",
                            count, action.action, str);
                success = false;
                break;
            }

            if let ActionResult::Failed(str) = result {
                eprintln!("Error running action index {} : {} - {}",
                            count, action.action, str);
                success = false;
                break;
            }

            if result != ActionResult::Success {
                eprintln!("Error running action index {} : {} - ...", count, action.action);
                success = false;
                break;
            }
        }

        if success {
            eprintln!("Successfully ran actions.");
        }
    }
}

