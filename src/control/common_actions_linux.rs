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

use crate::control::terminal_helpers_linux;

use super::control_actions::{ActionProvider, ActionResult, ControlAction, GenericError, SystemDetailsResult};
use super::control_common::ControlSession;

use rpassword::read_password;

// Note: this isn't strictly-speaking an Action which modifies any system state, it just returns details...
pub fn get_system_details(action_provider: &dyn ActionProvider, connection: &mut ControlSession) -> Result<SystemDetailsResult, GenericError> {
    // for the moment, assume the system's always going to be Linux,
    // but in the future we might need to more gracefully handle errors and attempt to work out
    // what platform it is...

    let full_command = "lsb_release --id --release";

    connection.conn.send_command(&action_provider.post_process_command(full_command));

    if connection.conn.get_previous_stdout_response().is_empty() {
        // stdout output was empty, which isn't expected...
        eprintln!("Invalid response from get_system_details() lsb_release command.");
        return Err(GenericError::CommandFailed("".to_string()));
    }

    let mut dist_id = String::new();
    let mut release = String::new();

    for line in connection.conn.get_previous_stdout_response().lines() {
        if let Some(value) = line.strip_prefix("Distributor ID:") {
            dist_id = value.trim().to_string();
        }
        else if let Some(value) = line.strip_prefix("Release:") {
            release = value.trim().to_string();
        }
    }

    // check we got something...
    // TODO: better than this?
    if dist_id.is_empty() || release.is_empty() {
        eprintln!("Unexpected response from get_system_details() lsb_release command. Expected values were not provided.");
        return Err(GenericError::CommandFailed("".to_string()));
    }

    Ok(SystemDetailsResult { distr_id: dist_id, release })
}

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
    if user.is_empty() {
        return ActionResult::InvalidParams("The 'username' parameter must be a valid string.".to_string());
    }

    let mut password = action.params.get_string_value("password").unwrap();
    if password == "$PROMPT" {
        eprintln!("Please enter password to set for new user '{}':", user);
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

    // work out what to do about any groups...

    // default group
    if let Some(default_group) = action.params.get_string_value("defaultGroup") {
        useradd_command_options.push_str(&format!("-g {} ", default_group));
    }

    // additional extra groups
    if action.params.has_value("extraGroups") {
        // there could be multiple...
        let extra_groups = action.params.get_values_as_vec_of_strings("extraGroups");
        useradd_command_options.push_str(&format!("-G {} ", extra_groups.join(",")));
    }

    // In theory, we should probably only optionally set this shell argument if the 'shell' param is set,
    // however that means in practice we often get '/bin/sh' shells by default which isn't great,
    // and having to mess around with '/etc/default/useradd' beforehand just to be "correct" seems
    // a bit silly, especially given the use-cases of Prod, so make an opinionated decision to have
    // '/bin/bash' as the default shell if the param's not specified.
    // If we do start supporting other platforms (MacOS / BSDs?), we might need to re-think this...
    let default_shell = action.params.get_string_value_with_default("shell", "/bin/bash");
    // however, special-case an empty string to allow not specifying this argument so the system default
    // can still be used if that is what's wanted...
    if !default_shell.is_empty() {
        useradd_command_options.push_str(&format!("-s {}", default_shell));
    }
    
    let useradd_full_command = format!("useradd {} {}", useradd_command_options, user);

    connection.conn.send_command(&action_provider.post_process_command(&useradd_full_command));

    // check response is nothing...
    if connection.conn.had_command_response() {
        return ActionResult::FailedCommand("Unexpected response from useradd command.".to_string());
    }

    // double make sure we don't add command to history here, even though post_process_command() should do it
    // if required.
    // TODO: only root can use chpasswd, and it will silently fail if the complexity requirement isn't met,
    //       which obviously isn't great...
//    let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
    let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
    connection.conn.send_command(&action_provider.post_process_command(&change_password_command));

    return ActionResult::Success;
}

pub fn systemctrl(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // validate params
    if !action.params.has_value("action") || !action.params.has_value("service") {
        return ActionResult::InvalidParams("".to_string());
    }

    let service = action.params.get_string_value("service").unwrap();
    let service_action = action.params.get_string_value("action").unwrap();

    let systemctrl_command = format!("systemctl {} {}", service_action, service);
    
    connection.conn.send_command(&action_provider.post_process_command(&systemctrl_command));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&systemctrl_command,
            action));
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
                    return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&ufw_command,
                        action));
                }
            }
        }

        let rules = action.params.get_values_as_vec_of_strings("rules");
        for rule in rules {
            let ufw_command = format!("ufw {}", rule);
            connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

            if connection.conn.did_exit_with_error_code() {
                return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&ufw_command,
                    action));
            }
        }

         if !start_first {
            if action.params.has_value("enabled") {
                let is_enabled = action.params.get_value_as_bool("enabled", true);
                let ufw_command = format!("ufw --force {}", if is_enabled { "enable" } else { "disable"});
                connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

                if connection.conn.did_exit_with_error_code() {
                    return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&ufw_command,
                        action));
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

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&timedatectl_command,
            action));
    }

    // TODO: also restart things like crond that might have been affected?

    return ActionResult::Success;
}

pub fn disable_swap(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let filename = action.params.get_string_value("filename");
    if filename.is_none() {
        return ActionResult::InvalidParams("The 'filename' parameter was not specified.".to_string());
    }
    let filename = filename.unwrap();

    // Note: filename can be '*' to delete all active swapfiles, however it needs to be quoted in YAML
    //       to be parsed correctly...

    // cat /proc/swaps
    let list_swapfiles_command = "cat /proc/swaps".to_string();
    connection.conn.send_command(&action_provider.post_process_command(&list_swapfiles_command));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&list_swapfiles_command,
            action));
    }

    // if there is already a swapfile configured, the stdout output should be more than one line...
    if connection.conn.get_previous_stdout_response().is_empty() {
        // stdout output was empty, which isn't expected...
        return ActionResult::FailedCommand("disableSwap error with unexpected response to 'cat /proc/swaps' command.".to_string());
    }

    let mut swapfile_names_to_delete = Vec::with_capacity(1);
    let swap_file_lines: Vec<&str> = connection.conn.get_previous_stdout_response().lines().collect();
    if swap_file_lines.len() == 1 {
        // we only have a single output line, which is (hopefully) the column headers, with no actual
        // swapfiles, so just return success...
        // TODO: we might still have a file on disk if something was done manually? (but how to know
        //       about it other than fstab? likely not worth worrying about?)
        return ActionResult::Success;
    }
    else if swap_file_lines.len() > 1 {
        for data_line in swap_file_lines.iter().skip(1) {
            let data_items: Vec<&str> = data_line.split_ascii_whitespace().collect();
            let swapfile_name = data_items[0].to_string();

            if swapfile_name == filename || filename == "*" {
                swapfile_names_to_delete.push(swapfile_name);
            }
        }
    }
    else {
        // Not really sure how we'd reach here unless the response was malformed...
        return ActionResult::FailedCommand("disableSwap error with unexpected response2".to_string());
    }

    if swapfile_names_to_delete.is_empty() {
        return ActionResult::FailedOther("disableSwap error: couldn't find specified swapfile to disable / delete.".to_string());
    }

    if filename == "*" {
        // disable them all
        let swapoff_command = "swapoff -a".to_string();
        connection.conn.send_command(&action_provider.post_process_command(&swapoff_command));

        if connection.conn.did_exit_with_error_code() {
            return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&swapoff_command,
                action));
        }
    }
    else {
        // only disable the one specified...
        // there should only be one in the list...
        let swap_file = &swapfile_names_to_delete[0];
        let swapoff_command = format!("swapoff {}", swap_file);

        connection.conn.send_command(&action_provider.post_process_command(&swapoff_command));

        if connection.conn.did_exit_with_error_code() {
            return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&swapoff_command,
                action));
        }
    }
    
    // TODO: delete the swapfile file (if it still exists)

    const FSTAB_FILE_PATH : &str = "/etc/fstab";

    // now edit /etc/fstab and comment out any line which configures the swapfile we found above...
    let fstab_string_contents = connection.conn.get_text_file_contents(FSTAB_FILE_PATH).unwrap();
    if fstab_string_contents.is_empty() {
        eprintln!("Error: /etc/fstab remote file has empty contents.");
        return ActionResult::FailedCommand("".to_string());
    }
    let fstab_contents_lines = fstab_string_contents.lines();

    let mut new_file_contents_lines = Vec::new();

    for line in fstab_contents_lines {
        let mut should_comment = false;
        for swap_file in &swapfile_names_to_delete {
            if line.contains(swap_file) {
                should_comment = true;
            }
        }

        if should_comment {
            // TODO: check if already commented out?
            new_file_contents_lines.push(format!("#{}", line));
        }
        else {
            new_file_contents_lines.push(line.to_string());
        }
    }

    // convert back to single string for entire file, and make sure we append a newline on the end...
    let new_file_contents_string = new_file_contents_lines.join("\n") + "\n";

    let stat_command = format!("stat {}", FSTAB_FILE_PATH);
    connection.conn.send_command(&action_provider.post_process_command(&stat_command));
    if let Some(strerr) = connection.conn.get_previous_stderr_response() {
        return ActionResult::FailedOther(format!("Error accessing remote fstab path: {}", strerr));
    }

    let stat_response = connection.conn.get_previous_stdout_response().to_string();
    // get the details from the stat call...
    let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);

    let mode;
    if let Some(stat_d) = stat_details {
        mode = i32::from_str_radix(&stat_d.0, 8).unwrap();
    }
    else {
        mode = 0o644;
        eprintln!("Can't extract stat details from file. Using 644 as default permissions mode.");
    }

    let send_res = connection.conn.send_text_file_contents(FSTAB_FILE_PATH, mode, &new_file_contents_string);
    if send_res.is_err() {
        return ActionResult::FailedOther("Error: failed to write modified /etc/fstab file.".to_string());
    }

    // now delete any of the swapfiles...
    // TODO: maybe wipe them optionally?
    for swap_file in swapfile_names_to_delete {
        let rm_command = format!("rm {}", swap_file);
        connection.conn.send_command(&action_provider.post_process_command(&rm_command));
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return ActionResult::FailedCommand(format!("Error deleting swapfile file: {}", strerr));
        }
    }
    
    return ActionResult::Success;
}

pub fn add_group(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // use groupadd and usermod commands which should be common across Linux distros...

    // validate params
    if !action.params.has_value("name") {
        return ActionResult::InvalidParams("The 'name' parameter was not specified.".to_string());
    }

    let group_name = action.params.get_string_value("name").unwrap();

    let groupadd_full_command = format!("groupadd {}", group_name);

    connection.conn.send_command(&action_provider.post_process_command(&groupadd_full_command));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&groupadd_full_command,
            action));
    }

    // now add users specified to the group
    // see if there's just a single group...
    if action.params.has_value("user") {
        let usermod_command = format!("usermod -aG {} {}", group_name, action.params.get_string_value_with_default("user", ""));
        connection.conn.send_command(&action_provider.post_process_command(&usermod_command));
        if connection.conn.did_exit_with_error_code() {
            return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&usermod_command,
                action));
        }
    }
    else if action.params.has_value("users") {
        // there's multiple
        let users = action.params.get_values_as_vec_of_strings("users");
        for user in users {
            let usermod_command = format!("usermod -aG {} {}", group_name, user);
            connection.conn.send_command(&action_provider.post_process_command(&usermod_command));
            if connection.conn.did_exit_with_error_code() {
                return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&usermod_command,
                    action));
            }
        }
    }

    return ActionResult::Success;
}

pub fn set_hostname(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // validate param
    if !action.params.has_value("hostname") {
        return ActionResult::InvalidParams("The 'hostname' parameter was not specified.".to_string());
    }

    // assume for the moment that systemd is installed, so hostnamectl can be used.

    let host_name = action.params.get_string_value("hostname").unwrap();

    let hostnamectrl_full_command = format!("hostnamectl set-hostname {}", host_name);

    connection.conn.send_command(&action_provider.post_process_command(&hostnamectrl_full_command));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&hostnamectrl_full_command,
            action));
    }

    // validate that it was set
    let hostnamectrl = "hostnamectl";
    connection.conn.send_command(&action_provider.post_process_command(&hostnamectrl));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&hostnamectrl,
            action));
    }

    // check we had output to stdout...
    if connection.conn.get_previous_stdout_response().is_empty() {
        // stdout output was empty, which isn't expected...
        return ActionResult::FailedCommand("setHostname error with unexpected response to 'hostnamectl' command.".to_string());
    }

    for line in connection.conn.get_previous_stdout_response().lines() {
        // we need to strip leading whitespace, as the title text per line
        // is right-aligned with spaces
        let stripped_line = line.trim_start();
        if let Some(result) = stripped_line.strip_prefix("Static hostname:") {
            if result.trim() == host_name {
                // it was set successfully...
                return ActionResult::Success;
            }
        }
    }

    // otherwise, something likely went wrong...
    return ActionResult::FailedCommand("setHostname action could not verify that hostname was set.".to_string());
}