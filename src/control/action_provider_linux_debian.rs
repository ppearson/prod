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

        let useradd_full_command = format!(" useradd {} {}", useradd_command_options, user);

        connection.conn.send_command(&useradd_full_command);

        // check response is nothing...
        if connection.conn.had_response() {
            return ActionResult::Failed("Unexpected response from useradd command.".to_string());
        }

//        let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
        let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
        connection.conn.send_command(&change_password_command);

        // now add user to any groups
        // see if there's just a single group...
        if params.params.has_value("group") {
            let usermod_command = format!(" usermod -aG {} {}", params.params.get_string_value_with_default("group", ""), user);
            connection.conn.send_command(&usermod_command);
        }
        else if params.params.has_value("groups") {
            // there's multiple
            let groups = params.params.get_values_as_vec_of_strings("groups");
            for group in groups {
                let usermod_command = format!(" usermod -aG {} {}", group, user);
                connection.conn.send_command(&usermod_command);
            }
        }

        eprintln!("Added user okay.");

        return ActionResult::Success;
    }

    fn create_directory(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // validate params
        if !params.params.has_value("path") {
            return ActionResult::InvalidParams;
        }

        let path_to_create = params.params.get_string_value("path").unwrap();
        let mnkdir_command = format!(" mkdir {}", path_to_create);
        connection.conn.send_command(&mnkdir_command);

        if let Some(permissions) = params.params.get_string_or_int_value_as_string("permissions") {
            let chmod_command = format!(" chmod {} {}", permissions, path_to_create);
            connection.conn.send_command(&chmod_command);
        }

        if let Some(owner) = params.params.get_string_value("owner") {
            let chown_command = format!(" chown {} {}", owner, path_to_create);
            connection.conn.send_command(&chown_command);
        }

        if let Some(group) = params.params.get_string_value("group") {
            let chgrp_command = format!(" chgrp {} {}", group, path_to_create);
            connection.conn.send_command(&chgrp_command);
        }

        // TODO: check for 'groups' as well to handle setting multiple...

        return ActionResult::Success;
    }

    fn install_packages(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

        let packages_string;
        if let Some(package) = params.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package.to_string();
        }
        else if params.params.has_value("packages") {
            let packages = params.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams;
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams;
        }

        let apt_get_command = format!(" apt-get -y install {}", packages_string);
        connection.conn.send_command(&apt_get_command);

        return ActionResult::Success;
    }

    fn systemctrl(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // validate params
        if !params.params.has_value("action") || !params.params.has_value("service") {
            return ActionResult::InvalidParams;
        }

        let service = params.params.get_string_value("service").unwrap();
        let action = params.params.get_string_value("action").unwrap();

        let systemctrl_command = format!("systemctl {} {}", action, service);
        
        connection.conn.send_command(&systemctrl_command);

        return ActionResult::Success;
    }

    fn firewall(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        let firewall_type = params.params.get_string_value_with_default("type", "ufw");
        if firewall_type != "ufw" {
            // only support this type for the moment...
            return ActionResult::InvalidParams;
        }

        // incredibly basic for the moment...
        let rules = params.params.get_values_as_vec_of_strings("rules");
        for rule in rules {
            let ufw_command = format!(" ufw {}", rule);
            connection.conn.send_command(&ufw_command);
        }

        if params.params.has_value("enabled") {
            let is_enabled = params.params.get_value_as_bool("enabled", true);
            let ufw_command = format!(" ufw {}", if is_enabled { "enable" } else { "disable"});
            connection.conn.send_command(&ufw_command);
        }

        return ActionResult::Success;
    }

}
