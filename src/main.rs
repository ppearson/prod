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

// REMOVE THIS!
//#![allow(warnings)]

use std::env;

mod common;
mod control;

mod params;

mod provision;

mod column_list_printer;

use control::control_manager::{ControlManager, CommandResult};
use control::control_actions::{ControlActions};

use provision::provision_common::{ProvisionActionType};
use provision::provision_manager::{ProvisionManager, ListType};
use provision::provision_params::{ProvisionParams, ParamValue};

enum MainType {
    Unknown,
    Provision,
    Control
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Error: prod requires at least one command line arg file.");
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
        if handle_provision_command(&args) {
            return;
        }
    }
    else if first_command == "control" && args.len() >= 3 {
        if handle_control_command(&args) {
            return;
        }
    }

    eprintln!("Error: Didn't understand command line args...");
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
        let provision_params = ProvisionParams::from_file(&next_arg).unwrap();

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
    return false;
}

// return value indicates whether function handled input or not. If true it did,
// if false, it fell through...
pub fn handle_control_command(args: &Vec<String>) -> bool {
    let next_arg = &args[2];
    let host = next_arg;

    let control_manager = ControlManager::new();

    if next_arg.contains('.') && args.len() == 3 {
        // likely a control/action file
        // TODO: error handling!
        let control_actions = ControlActions::from_file(next_arg).unwrap();

        control_manager.perform_actions(&control_actions);

        return true;
    }
    else if args.len() >= 4 {
        // next arg is command to run remotely...
        let command_str = &args[3];
 
        let res = control_manager.run_command(host, command_str);
        match res {
            CommandResult::ErrorCantConnect(err) => {
                eprintln!("Error: can't connect to host: {}, {}...", host, err);
            },
            CommandResult::ErrorAuthenticationIssue(err) => {
                eprintln!("Error: can't authenticate with host: {}, {}...", host, err);
            },
            CommandResult::Failed(err) => {
                eprintln!("Error: failed to run remote command: {}...", err);
            },
            CommandResult::CommandRunOkay(result) => {
                println!("Command executed okay. Response:\n{}\n", result);
            }
        }
      
        return true;
    }

    return false;
}