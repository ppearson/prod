/*
 Prod
 Copyright 2021 Peter Pearson.
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

use super::control_actions::{ActionProvider, ActionResult, ControlAction};
use super::control_common::{ControlConnection};

use rpassword::read_password;

pub struct AProviderLinuxDebian {
    
}

impl AProviderLinuxDebian {
    pub fn new() -> AProviderLinuxDebian {
        AProviderLinuxDebian {  }
    }
}

impl ActionProvider for AProviderLinuxDebian {
    fn name(&self) -> String {
        return "linux_debian".to_string();
    }

    fn add_user(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // validate params
        if !params.params.has_value("username") || !params.params.has_value("password") {
            return ActionResult::InvalidParams;
        }


        let mut useradd_command_options = String::new();

        let user = params.params.get_string_value("username").unwrap();
        let mut password = params.params.get_string_value("password").unwrap();
        if password == "$PROMPT" {
            eprintln!("Please enter password to set for user:");
            password = read_password().unwrap();
        }

        let create_home = params.params.get_value_as_bool("createHome", true);

        if create_home {
            useradd_command_options.push_str("-m ");
        }
        else {
            // do not create user's home group...
            useradd_command_options.push_str("-M ");
        }

        let shell = params.params.get_string_value_with_default("shell", "/bin/bash");
        useradd_command_options.push_str(&format!("-s {}", shell));

        let useradd_full_command = format!("useradd {} {}", useradd_command_options, user);

        

        eprintln!("Running command: '{}'", useradd_full_command);

        connection.conn.send_command(&useradd_full_command);

        // check response is nothing...
        if connection.conn.had_response() {
            return ActionResult::Failed("Unexpected response from useradd command.".to_string());
        }


//        let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
        let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
        
        connection.conn.send_command(&change_password_command);

        eprintln!("Added user okay.: {}", connection.conn.prev_std_out);

        return ActionResult::Success;
    }

}
