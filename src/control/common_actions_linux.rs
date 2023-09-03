/*
 Prod
 Copyright 2021-2023 Peter Pearson.
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
use super::control_common::{ControlSession};

use rpassword::read_password;

pub fn add_user(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // use useradd command which should be common across Linux distros...

    // validate params
    if !action.params.has_value("username") {
        return ActionResult::InvalidParams("The 'username' parameter was not specified.".to_string());
    }
    if !action.params.has_value("password") {
        return ActionResult::InvalidParams("The 'password' parameter was not specified.".to_string());
    }

    let mut useradd_command_options = String::new();

    let user = action.params.get_string_value("username").unwrap();
    let mut password = action.params.get_string_value("password").unwrap();
    if password == "$PROMPT" {
        eprintln!("Please enter password to set for user:");
        password = read_password().unwrap();
    }

    let create_home = action.params.get_value_as_bool("createHome", true);

    if create_home {
        useradd_command_options.push_str("-m ");
    }
    else {
        // do not create user's home group...
        useradd_command_options.push_str("-M ");
    }

    let shell = action.params.get_string_value_with_default("shell", "/bin/bash");
    useradd_command_options.push_str(&format!("-s {}", shell));

    let useradd_full_command = format!("useradd {} {}", useradd_command_options, user);

    connection.conn.send_command(&action_provider.post_process_command(&useradd_full_command));

    // check response is nothing...
    if connection.conn.had_command_response() {
        return ActionResult::Failed("Unexpected response from useradd command.".to_string());
    }

    // double make sure we don't add command to history here, even though post_process_command() should do it
    // if required.
//    let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
    let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
    connection.conn.send_command(&action_provider.post_process_command(&change_password_command));

    let mut check_no_response = false;
    // now add user to any groups
    // see if there's just a single group...
    if action.params.has_value("group") {
        let usermod_command = format!("usermod -aG {} {}", action.params.get_string_value_with_default("group", ""), user);
        connection.conn.send_command(&action_provider.post_process_command(&usermod_command));
        check_no_response = true;
    }
    else if action.params.has_value("groups") {
        // there's multiple
        let groups = action.params.get_values_as_vec_of_strings("groups");
        for group in groups {
            let usermod_command = format!("usermod -aG {} {}", group, user);
            connection.conn.send_command(&action_provider.post_process_command(&usermod_command));
        }
        check_no_response = true;
    }

    if check_no_response {
    // check response is nothing...
        if connection.conn.had_command_response() {
            return ActionResult::Failed(format!("Unexpected response from usermod command: {}", connection.conn.get_previous_stderr_response().unwrap_or("")));
        }
    }

    return ActionResult::Success;
}

pub fn systemctrl(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // validate params
    if !action.params.has_value("action") || !action.params.has_value("service") {
        return ActionResult::InvalidParams("".to_string());
    }

    let service = action.params.get_string_value("service").unwrap();
    let action = action.params.get_string_value("action").unwrap();

    let systemctrl_command = format!("systemctl {} {}", action, service);
    
    connection.conn.send_command(&action_provider.post_process_command(&systemctrl_command));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::Failed(format!("Unexpected response from '{}' command: {}", systemctrl_command,
                connection.conn.get_previous_stderr_response().unwrap_or("")));
    }

    return ActionResult::Success;
}

pub fn firewall(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction, start_first: bool) -> ActionResult {
    let firewall_type = action.params.get_string_value_with_default("type", "ufw");
    if firewall_type == "ufw" {
        // incredibly basic for the moment...
        // in theory we should probably be more type-specific, and 'schema'd', but given there
        // are aliases for rules, it'd be quite complicated to handle that I think, so better
        // for the moment to allow freeform strings...

        // according to ufw's man, adding rules before ufw is enabled is supported (and works fine under debian/ubuntu),
        // but fedora doesn't seem to like this first time around after install, and you seemingly need to enable ufw before
        // it will accept any rules, hence the below conditional logic...
        if start_first {
            if action.params.has_value("enabled") {
                let is_enabled = action.params.get_value_as_bool("enabled", true);
                let ufw_command = format!("ufw --force {}", if is_enabled { "enable" } else { "disable"});
                connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

                if connection.conn.did_exit_with_error_code() {
                    return ActionResult::Failed(format!("Unexpected response from '{}' command: {}", ufw_command,
                         connection.conn.get_previous_stderr_response().unwrap_or("")));
                }
            }
        }

        let rules = action.params.get_values_as_vec_of_strings("rules");
        for rule in rules {
            let ufw_command = format!("ufw {}", rule);
            connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

            if connection.conn.did_exit_with_error_code() {
                return ActionResult::Failed(format!("Unexpected response from '{}' command: {}", ufw_command,
                        connection.conn.get_previous_stderr_response().unwrap_or("")));
            }
        }

         if !start_first {
            if action.params.has_value("enabled") {
                let is_enabled = action.params.get_value_as_bool("enabled", true);
                let ufw_command = format!("ufw --force {}", if is_enabled { "enable" } else { "disable"});
                connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

                if connection.conn.did_exit_with_error_code() {
                    return ActionResult::Failed(format!("Unexpected response from '{}' command: {}", ufw_command,
                            connection.conn.get_previous_stderr_response().unwrap_or("")));
                }
            }
        }
    }
    else {
        // only support this type for the moment...
        return ActionResult::InvalidParams("Invalid firewall type param".to_string());
    }

    return ActionResult::Success;
}

pub fn set_time_zone(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let time_zone = action.params.get_string_value("timeZone");
    if time_zone.is_none() {
        return ActionResult::InvalidParams("The 'timeZone' parameter was not specified.".to_string());
    }
    let time_zone = time_zone.unwrap();

    // "UTC", "Pacific/Auckland", "Europe/London"

    let timedatectl_command = format!("timedatectl {}", time_zone);
    connection.conn.send_command(&action_provider.post_process_command(&timedatectl_command));

    if let Some(str) = connection.conn.get_previous_stderr_response() {
        eprintln!("set_time_zone error: {}", str);
        return ActionResult::Failed(str.to_string());
    }

    // TODO: also restart things like crond that might have been affected?

    return ActionResult::Success;
}
