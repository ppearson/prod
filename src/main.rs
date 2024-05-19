/*
 Prod
 Copyright 2021-2024 Peter Pearson.
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

use std::env;

mod common;
mod control;

mod params;

mod provision;

mod column_list_printer;

use control::control_manager::{ControlManager, CommandResult, ControlGeneralParams};
use control::control_actions::ControlActions;

use provision::provision_common::ProvisionActionType;
use provision::provision_manager::{ProvisionManager, ListType};
use provision::provision_params::{ProvisionParams, ParamValue};

/*
enum MainType {
    Unknown,
    Provision,
    Control
}
*/

fn print_help() {
    eprintln!("prod usage:");
    eprintln!();
    eprintln!("prod provision list <provider> <plans/regions/oss>         : list available provision items");
    eprintln!("prod provision <provision_file>                            : run provision script");
    eprintln!("prod provision deleteInstance <provider> <instance_id>     : delete instance");
    
    eprintln!();

    eprintln!("prod control [-retry] <control_script_file>     : Run control script file");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Error: prod requires at least one command line arg file.");
        print_help();
        return
    }

    // work out if we have override env variables set for type...
/*    
    let mut main_type = MainType::Unknown;
    if let Ok(val) = std::env::var("PROD_TYPE_OVERRIDE") {
        if val == "provision" {
            main_type = MainType::Provision;
        }
        else if val == "control" {
            main_type = MainType::Control;
        }
    }
*/
    // and for provider...
/*
    let mut provider_override: Option<String> = None;
    if let Ok(String) = std::env::var("PROD_PROVIDER_OVERRIDE") {
        provider_override = Some(String);
    }
*/

    let first_command = &args[1];
    if first_command.contains('.') {
        // looks like a file, so...
        // TODO: automatically run it based off detecting what it is...
    }
    else if first_command == "provision" && args.len() >= 3 {
        handle_provision_command(&args);
        return;
    }
    else if first_command == "control" && args.len() >= 3 {
        handle_control_command(&args);
        return;
    }
    else if first_command.contains("help") {
        print_help();
        return;
    }

    eprintln!("Error: Didn't understand command line args...");
    print_help();
}

// return value indicates whether function handled input or not. If true it did,
// if false, it fell through...
pub fn handle_provision_command(args: &Vec<String>) -> bool {
    let next_arg = &args[2];
    let provision_manager = ProvisionManager::new();
    let dry_run = false;
    if next_arg.contains('.') && args.len() == 3 {
        // likely a provision file
        // TODO: error handling!
        let provision_params = ProvisionParams::from_file(next_arg).unwrap();

        if provision_params.provider.is_empty() {
            eprintln!("Error: no provider was specified in file: {}", next_arg);
            return true;
        }

        if provision_params.action == ProvisionActionType::NotSet {
            eprintln!("Error: no action was specified in file: {}", next_arg);
            return true;
        }

        let _response = provision_manager.perform_action(&provision_params, dry_run);

        return true;
    }
    else {
        // hopefully a command + provider
        // TODO: swap command + provider order around given we will allow overriding provider
        //       with env variable in future, and might make more contextual sense?
        let command = &args[2];
        if command == "list" && args.len() >= 4 {
            let provider = &args[3];
            let mut list_type = ListType::Regions;
            if args.len() > 4 {
                list_type = match args[4].as_str() {
                    "plans" => ListType::Plans,
                    "regions" => ListType::Regions,
                    "os" | "oss" => ListType::OSs,
                    _ => ListType::Regions
                };
            }
            provision_manager.list_available(provider, list_type);
            return true;
        }
        else if (command == "delInstance" || command == "deleteInstance") && args.len() > 4 {
            let provider = &args[3];
            let instance_id = &args[4];

            let mut params = ProvisionParams::from_details(provider, ProvisionActionType::DeleteInstance);
            params.values.insert("instance_id".to_string(), ParamValue::StringVal(instance_id.to_string()));
            let _response = provision_manager.perform_action(&params, dry_run);

            return true;
        }
        else {
            eprintln!("Unrecognised command string: '{}'", command);
        }
    }

    // didn't handle input...
    false
}

// return value indicates whether function handled input or not. If true it did,
// if false, it fell through...
pub fn handle_control_command(args: &[String]) -> bool {
    let control_manager = ControlManager::new();

    let mut general_params = ControlGeneralParams::new();

    enum ControlType {
        Unknown,
        ManualCommand(String, String), // hostname, command
        ActionsScript(String),
    }

    let mut run_kind = ControlType::Unknown;

    let mut arg_iter = args.iter().skip(2).enumerate().peekable();
    while let Some((_idx, arg)) = arg_iter.next() {
        if arg == "--command" {
            // we want to run a manual command on the specified host...

            // check the next arg exists and is (hopefully) a hostname...
            if let Some(hostname) = arg_iter.next() {
                // then next arg should be the single (quoted) command to run
                if let Some(command_str) = arg_iter.next() {
                    run_kind = ControlType::ManualCommand(hostname.1.to_string(), command_str.1.to_string());
                    break;
                }
                else {
                    // error out...
                    break;
                }
            }
            else {
                eprintln!("Error: expected a hostname arg after the '--command' arg.");
                return false;
            }
        }
        else if let Some(flag_string) = arg.strip_prefix('-') {
            match flag_string {
                "retry"    => {
                    general_params.retry = true;
                },
                _  => {
                    eprintln!("Warning: unrecognised command flag: {}", arg);
                }
            }
        }
        else {
            // it's a full string arg, so hopefully control/action script file to open and perform...

            let filename = arg;
            run_kind = ControlType::ActionsScript(filename.to_string());
        }
    }

    match run_kind {
        ControlType::ManualCommand(hostname, command_str) => {
            // run the single manual command on the host requested...

            let res = control_manager.run_command(&hostname, &command_str);
            match res {
                CommandResult::ErrorCantConnect(err) => {
                    eprintln!("Error: can't connect to host: {}, {}...", hostname, err);
                },
                CommandResult::ErrorAuthenticationIssue(err) => {
                    eprintln!("Error: can't authenticate with host: {}, {}...", hostname, err);
                },
                CommandResult::Failed(err) => {
                    eprintln!("Error: failed to run remote command: {}...", err);
                },
                CommandResult::CommandRunOkay(result) => {
                    println!("Command executed okay. Response:\n{}\n", result);
                }
            }
        },
        ControlType::ActionsScript(script_file) => {
            // run the actual script...

            let file_read_res = ControlActions::from_file(&script_file);
            if let Ok(control_actions) = file_read_res {
                control_manager.perform_actions(&control_actions, general_params);
            }
            else {
                eprintln!("Error loading Actions file.");
                return false;
            }

            return true;
        },
        _   => {
            eprintln!("Error running control command, invalid type status.");
            return false;
        }
    }

    false
}