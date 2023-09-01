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

pub struct ControlGeneralParams {
    pub retry:      bool,
}

impl ControlGeneralParams {
    pub fn new() -> ControlGeneralParams {
        ControlGeneralParams { retry: false }
    }
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

        let target_host = host.to_string();

        println!("Enter password:");
        let password = read_password().unwrap();

        let username = "peter";

        let control_session_user_auth = ControlSessionUserAuth::UserPass(UserAuthUserPass::new(&username, &password));

        let session_params = ControlSessionParams::new(&target_host, 22, control_session_user_auth, true);

#[cfg(feature = "openssh")]
        let connection = ControlSession::new_openssh(session_params);
#[cfg(feature = "sshrs")]
        let connection = ControlSession::new_sshrs(session_params);
#[cfg(not(any(feature = "openssh", feature = "sshrs")))]
        let connection = ControlSession::new_dummy_debug(session_params);

        if connection.is_none() {
            eprintln!("Error connecting to hostname...");
            return CommandResult::ErrorCantConnect("".to_string());
        }
        let mut connection = connection.unwrap();

        connection.conn.send_command(command);

        return CommandResult::CommandRunOkay(connection.conn.get_previous_stdout_response().to_string());
    }

    pub fn perform_actions(&self, actions: &ControlActions, general_params: ControlGeneralParams) {
        if actions.actions.is_empty() {
            eprintln!("Error: no actions specified.");
        }

        // TODO: come up with a better way of handling this partial initialisation / ordering dilema to work
        //       out if a provider exists before querying for usernames and passwords...
        let mut session_params = ControlSessionParams::new("",
                                                           22,
                                                           actions.auth.clone(), true);

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
        let mut port: Option<u32> = None;
        if actions.host.is_empty() || actions.host == "$PROMPT" {
            eprintln!("Please enter hostname to connect to:");
            std::io::stdin().read_line(&mut hostname).expect("Error reading hostname from std input");
            hostname = hostname.trim().to_string();

            if let Some(split_pair) = hostname.split_once(':') {
                // we have a hostname and a port number...
                // use temp variable to cache value to prevent modifying backing string...
                let tmp_hostname = split_pair.0.to_string();
                
                let parsed_port = split_pair.1.parse::<u32>();
                if let Err(_err) = parsed_port {
                    eprintln!("Error parsing suffix port number after hostname: {}", split_pair.1);
                }
                else {
                    port = Some(parsed_port.unwrap());
                }

                hostname = tmp_hostname;
            }

            asked_for_hostname = true;
        }
        else {
            hostname = actions.host.clone();
            port = actions.port;
        }

        // connect to host
        let target_host = hostname;
        let target_port = port;

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
                    eprintln!("Enter password for user '{}' on host '{}':", &username, &target_host);
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
                    eprintln!("Enter key passphrase for user '{}' on host '{}':", &username, &target_host);
                }
                else {
                    eprintln!("Enter key passphrase:");
                }
                publickey.passphrase = read_password().unwrap();
            }
        }

        let mut connection;
        // always loop for retry logic, but we break out normally on success...
        const RETRY_LIMIT: usize = 15;
        let mut retry_count = 0;

        let port_number = target_port.unwrap_or(22);

        loop {
            eprintln!("Connecting to {}:{}...", target_host, port_number);

            // Now configure ControlSessionParams properly here...
            // TODO: as above, not really happy with this, but there's various "not great" ways of solving the issue
            //       I don't like, so I'm happier (only just) with this for the moment...
            session_params = ControlSessionParams::new(&target_host, port_number, auth.clone(), true);

#[cfg(feature = "openssh")]
            let inner_connection = ControlSession::new_openssh(session_params);
#[cfg(feature = "sshrs")]
            let inner_connection = ControlSession::new_sshrs(session_params);
#[cfg(not(any(feature = "openssh", feature = "sshrs")))]
            let inner_connection = ControlSession::new_dummy_debug(session_params);

            if let Some(connection_result) = inner_connection {
                // we were successful, so save the result, and break out of the retry loop.
                connection = connection_result;
                break;
            }

            // otherwise, we had an error, so retry if requested...
            if general_params.retry {
                // we want to retry automatically after a pause...
                // TODO: only if it was a connection failure!!
                //       we probably don't want to keep retrying if it was an auth failure (account lockout!)
                if retry_count <= RETRY_LIMIT {
                    eprintln!("Connection failed... will retry in 30 secs...");
                    retry_count += 1;
                }
                else {
                    eprintln!("Connection failed after: {} retry attempts, will abort.", retry_count);
                    return
                }
                std::thread::sleep(std::time::Duration::from_secs(30));
                eprintln!("Retrying connection...");
            }
            else {
                // we don't want to retry, just error...
                // TODO: sprinkling this 22 default everywhere isn't great... maybe make it non-optional
                //       in the params struct so it's just default constructed with 22, and overridden
                //       if necessary?
                eprintln!("Error connecting to: {}:{}...", target_host, target_port.unwrap_or(22));
                return;
            }
        }

        eprintln!("Connected successfully...");

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
                ControlActionType::RemoveDirectory => {
                    provider.remove_directory(&mut connection, action)
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
                ControlActionType::RemoveFile => {
                    provider.remove_file(&mut connection, action)
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

