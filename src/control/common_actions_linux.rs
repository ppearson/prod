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

use super::control_actions::{ActionProvider, ActionError, ControlAction, GenericError, SystemDetailsResult};
use super::control_common::ControlSession;

use rpassword::read_password;

// Note: this isn't strictly-speaking an Action which modifies any system state, it just returns details...
pub fn get_system_details(action_provider: &dyn ActionProvider, connection: &mut ControlSession
) -> Result<SystemDetailsResult, GenericError> {
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

pub fn add_user(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // use useradd command which should be common across Linux distros...

    let mut useradd_command_options = String::new();

    let user = action.get_required_string_param("username")?;
    let mut password = action.get_required_string_param("password")?;
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
        return Err(ActionError::FailedCommand("Unexpected response from useradd command.".to_string()));
    }

    // double make sure we don't add command to history here, even though post_process_command() should do it
    // if required.
    // TODO: only root can use chpasswd, and it will silently fail if the complexity requirement isn't met,
    //       which obviously isn't great...
//    let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
    let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
    connection.conn.send_command(&action_provider.post_process_command(&change_password_command));

    Ok(())
}

pub fn systemctrl(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let service = action.get_required_string_param("service")?;
    let service_action = action.get_required_string_param("action")?;

    let systemctrl_command = format!("systemctl {} {}", service_action, service);
    
    connection.conn.send_command(&action_provider.post_process_command(&systemctrl_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&systemctrl_command,
            action)));
    }

    Ok(())
}

pub fn firewall(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction, start_first: bool
) -> Result<(), ActionError> {
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

                // we can't just rely on stderr being useful here, i.e. if ufw wasn't installed or something...
                if connection.conn.did_exit_with_error_code() {
                    // stdout can sometimes be useful though, so look for obvious things to be a bit more helpful
                    if connection.conn.get_previous_stdout_response().contains("ufw: command not found") {
                        eprintln!("Error in 'firewall' action: 'ufw' does not seem to be installed.");
                    }
                    return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&ufw_command,
                        action)));
                }
            }
        }

        let rules = action.params.get_values_as_vec_of_strings("rules");
        for rule in rules {
            let ufw_command = format!("ufw {}", rule);
            connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

            // we can't just rely on stderr being useful here when things fail, i.e. if ufw wasn't installed or something,
            // but the exit code should always be indicative...
            if connection.conn.did_exit_with_error_code() {
                // stdout can sometimes be useful though, so look for obvious things to be a bit more helpful
                if connection.conn.get_previous_stdout_response().contains("ufw: command not found") {
                    eprintln!("Error in 'firewall' action: 'ufw' does not seem to be installed.");
                }
                return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&ufw_command,
                    action)));
            }
        }

         if !start_first {
            if action.params.has_value("enabled") {
                let is_enabled = action.params.get_value_as_bool("enabled", true);
                let ufw_command = format!("ufw --force {}", if is_enabled { "enable" } else { "disable"});
                connection.conn.send_command(&action_provider.post_process_command(&ufw_command));

                if connection.conn.did_exit_with_error_code() {
                    return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&ufw_command,
                        action)));
                }
            }
        }
    }
    else {
        // only support this type for the moment...
        return Err(ActionError::InvalidParams("Invalid firewall type param".to_string()));
    }

    Ok(())
}

pub fn set_time_zone(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let time_zone = action.get_required_string_param("timeZone")?;
   
    // "UTC", "Pacific/Auckland", "Europe/London"

    let timedatectl_command = format!("timedatectl {}", time_zone);
    connection.conn.send_command(&action_provider.post_process_command(&timedatectl_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&timedatectl_command,
            action)));
    }

    // TODO: also restart things like crond that might have been affected?

    Ok(())
}

pub fn disable_swap(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let filename = action.get_required_string_param("filename")?;

    // Note: filename can be '*' to delete all active swapfiles, however it needs to be quoted in YAML
    //       to be parsed correctly...

    // cat /proc/swaps
    let list_swapfiles_command = "cat /proc/swaps".to_string();
    connection.conn.send_command(&action_provider.post_process_command(&list_swapfiles_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&list_swapfiles_command,
            action)));
    }

    // if there is already a swapfile configured, the stdout output should be more than one line...
    if connection.conn.get_previous_stdout_response().is_empty() {
        // stdout output was empty, which isn't expected...
        return Err(ActionError::FailedCommand("disableSwap error with unexpected response to 'cat /proc/swaps' command.".to_string()));
    }

    let mut swapfile_names_to_delete = Vec::with_capacity(1);
    let swap_file_lines: Vec<&str> = connection.conn.get_previous_stdout_response().lines().collect();
    if swap_file_lines.len() == 1 {
        // we only have a single output line, which is (hopefully) the column headers, with no actual
        // swapfiles, so just return success...
        // TODO: we might still have a file on disk if something was done manually? (but how to know
        //       about it other than fstab? likely not worth worrying about?)
        return Ok(());
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
        return Err(ActionError::FailedCommand("disableSwap error with unexpected response2".to_string()));
    }

    if swapfile_names_to_delete.is_empty() {
        return Err(ActionError::FailedOther("disableSwap error: couldn't find specified swapfile to disable / delete.".to_string()));
    }

    if filename == "*" {
        // disable them all
        let swapoff_command = "swapoff -a".to_string();
        connection.conn.send_command(&action_provider.post_process_command(&swapoff_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&swapoff_command,
                action)));
        }
    }
    else {
        // only disable the one specified...
        // there should only be one in the list...
        let swap_file = &swapfile_names_to_delete[0];
        let swapoff_command = format!("swapoff {}", swap_file);

        connection.conn.send_command(&action_provider.post_process_command(&swapoff_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&swapoff_command,
                action)));
        }
    }
    
    // TODO: delete the swapfile file (if it still exists)

    const FSTAB_FILE_PATH : &str = "/etc/fstab";

    // now edit /etc/fstab and comment out any line which configures the swapfile we found above...
    let fstab_string_contents = connection.conn.get_text_file_contents(FSTAB_FILE_PATH).unwrap();
    if fstab_string_contents.is_empty() {
        eprintln!("Error: /etc/fstab remote file has empty contents.");
        return Err(ActionError::FailedCommand("".to_string()));
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
        return Err(ActionError::FailedOther(format!("Error accessing remote fstab path: {}", strerr)));
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
        return Err(ActionError::FailedOther("Error: failed to write modified /etc/fstab file.".to_string()));
    }

    // now delete any of the swapfiles...
    // TODO: maybe wipe them optionally?
    for swap_file in swapfile_names_to_delete {
        let rm_command = format!("rm {}", swap_file);
        connection.conn.send_command(&action_provider.post_process_command(&rm_command));
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedCommand(format!("Error deleting swapfile file: {}", strerr)));
        }
    }
    
    Ok(())
}

pub fn add_group(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // use groupadd and usermod commands which should be common across Linux distros...
    let group_name = action.get_required_string_param("name")?;

    let groupadd_full_command = format!("groupadd {}", group_name);

    connection.conn.send_command(&action_provider.post_process_command(&groupadd_full_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&groupadd_full_command,
            action)));
    }

    // now add users specified to the group
    // see if there's just a single group...
    if action.params.has_value("user") {
        let usermod_command = format!("usermod -aG {} {}", group_name, action.params.get_string_value_with_default("user", ""));
        connection.conn.send_command(&action_provider.post_process_command(&usermod_command));
        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&usermod_command,
                action)));
        }
    }
    else if action.params.has_value("users") {
        // there's multiple
        let users = action.params.get_values_as_vec_of_strings("users");
        for user in users {
            let usermod_command = format!("usermod -aG {} {}", group_name, user);
            connection.conn.send_command(&action_provider.post_process_command(&usermod_command));
            if connection.conn.did_exit_with_error_code() {
                return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&usermod_command,
                    action)));
            }
        }
    }

    Ok(())
}

pub fn set_hostname(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // assume for the moment that systemd is installed, so hostnamectl can be used.
    let host_name = action.get_required_string_param("hostname")?;

    let hostnamectrl_full_command = format!("hostnamectl set-hostname {}", host_name);

    connection.conn.send_command(&action_provider.post_process_command(&hostnamectrl_full_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&hostnamectrl_full_command,
            action)));
    }

    // validate that it was set
    let hostnamectrl = "hostnamectl";
    connection.conn.send_command(&action_provider.post_process_command(&hostnamectrl));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&hostnamectrl,
            action)));
    }

    // check we had output to stdout...
    if connection.conn.get_previous_stdout_response().is_empty() {
        // stdout output was empty, which isn't expected...
        return Err(ActionError::FailedCommand("setHostname error with unexpected response to 'hostnamectl' command.".to_string()));
    }

    for line in connection.conn.get_previous_stdout_response().lines() {
        // we need to strip leading whitespace, as the title text per line
        // is right-aligned with spaces
        let stripped_line = line.trim_start();
        if let Some(result) = stripped_line.strip_prefix("Static hostname:") {
            if result.trim() == host_name {
                // it was set successfully...
                return Ok(())
            }
        }
    }

    // otherwise, something likely went wrong...
    Err(ActionError::FailedCommand("setHostname action could not verify that hostname was set.".to_string()))
}

pub fn create_systemd_service(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let service_name = action.get_required_string_param("name")?;
    let description = action.get_required_string_param("description")?;
    let user = action.get_required_string_param("user")?;
    let exec_start = action.get_required_string_param("execStart")?;

    // Note: docs here: https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#Options

    let unit_service_file_path = format!("/etc/systemd/system/{}.service", service_name);

    // TODO: see if it exists already?

    let mut file_content = format!("[Unit]\nDescription={}\n", description);

    if let Some(after) = action.params.get_string_value("after") {
        file_content.push_str(&format!("After={}\n", after));
    }
    if let Some(before) = action.params.get_string_value("before") {
        file_content.push_str(&format!("Before={}\n", before));
    }  

    file_content.push('\n');

    // default 

    file_content.push_str(&format!("[Service]\nType=simple\nUser={}\nExecStart={}\n",
        user,
        exec_start));
    
    if let Some(exec_reload) = action.params.get_string_value("execRestart") {
        file_content.push_str(&format!("ExecRestart={}\n", exec_reload));
    }

    if let Some(exec_stop) = action.params.get_raw_value("execStop") {
        file_content.push_str(&format!("ExecStop={}\n", exec_stop));
    }
    
    // TODO: make this configurable...
    file_content.push_str(&format!("Restart=always\nRestartSec=2\n"));
    file_content.push('\n');

    // and this...
    file_content.push_str("[Install]\nWantedBy=multi-user.target\n");

    // Note: currently with ssh-rs being used (which doesn't support setting target file mode perms), this
    //       will only be useable with the 'root' user being enabled.
    let res = connection.conn.send_text_file_contents(&unit_service_file_path, 0o644, &file_content);
    if let Err(err) = res {
        return Err(ActionError::FailedCommand(format!("Error creating remote file for new service: '{}', error: {}",
        unit_service_file_path, err)));
    }

    // reload it

    let reload_command = "sudo systemctl daemon-reload";
    connection.conn.send_command(&action_provider.post_process_command(reload_command));
    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(reload_command,
            action)));
    }

    // check to see if we've been told not to start it now
    let should_start = action.params.get_value_as_bool("startNow", true);
    if should_start {
        // now start it
        let systemctrl_start_command = format!("systemctl start {}", service_name);
        connection.conn.send_command(&action_provider.post_process_command(&systemctrl_start_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&systemctrl_start_command,
                action)));
        }
    }

    // now enable it (think this starts it on boot... maybe that should be conditional, i.e. connected with the 'WantedBy' bit?)
    let systemctrl_enable_command = format!("systemctl enable {}", service_name);
    connection.conn.send_command(&action_provider.post_process_command(&systemctrl_enable_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&systemctrl_enable_command,
            action)));
    }

    if should_start {
        // check that it did start...
        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(
                "Error: systemctl status did not return that the newly created service was actually started.".to_string()));
        }
    }
    else {
        // TODO: maybe we can check some other status to check things are valid in the absence of starting it,
        //       although maybe that's not needed?
    }

    Ok(())
}