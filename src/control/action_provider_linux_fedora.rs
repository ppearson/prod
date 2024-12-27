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

use super::common_actions_linux;
use super::common_actions_unix;

use super::control_actions::{ActionProvider, ActionResult, ControlAction, GenericError, SystemDetailsResult};
use super::control_common::{ControlSession, ControlSessionParams};

pub struct AProviderLinuxFedora {
    // params which give us some hints as to context of session, i.e. username - sudo vs root, etc.
    session_params: ControlSessionParams,
}

impl AProviderLinuxFedora {
    pub fn new(session_params: ControlSessionParams) -> AProviderLinuxFedora {
        AProviderLinuxFedora { session_params }
    }

    pub fn name() -> String {
        return "linux_fedora".to_string();
    }
}

impl ActionProvider for AProviderLinuxFedora {
    fn name(&self) -> String {
        return "linux_fedora".to_string();
    }

    fn get_session_params(&self) -> Option<&ControlSessionParams> {
        return Some(&self.session_params);
    }

    fn generic_command(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::generic_command(self, connection, action);
    }

    // this is not really an Action, as it doesn't modify anything, it just returns values, but...
    fn get_system_details(&self, connection: &mut ControlSession) -> Result<SystemDetailsResult, GenericError> {
        return common_actions_linux::get_system_details(self, connection);
    }

    fn add_user(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::add_user(self, connection, action);
    }

    fn create_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::create_directory(self, connection, action);
    }

    fn remove_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::remove_directory(self, connection, action);
    }

    fn install_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let packages_string;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if action.params.has_value("packages") {
            let packages = action.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string());
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams("The resulting 'packages' string list was empty.".to_string());
        }

        // by default, update the list of packages, as with some providers,
        // this needs to be done first, otherwise packages can't be found...
        let update_packages = action.params.get_value_as_bool("update", true);
        if update_packages {
            let dnf_command = "dnf -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&dnf_command));
        }

        let dnf_command = format!("dnf -y install {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&dnf_command));

        if connection.conn.did_exit_with_error_code() {
            return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&dnf_command,
                action));
        }

        return ActionResult::Success;
    }

    fn remove_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let packages_string;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if action.params.has_value("packages") {
            let packages = action.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string());
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams("The resulting 'packages' string list was empty.".to_string());
        }

        let dnf_command = format!("dnf -y remove {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&dnf_command));

        let ignore_failure = action.params.get_value_as_bool("ignoreFailure", false);

        if connection.conn.did_exit_with_error_code() {
            if !ignore_failure {
                return ActionResult::FailedCommand(connection.conn.return_failed_command_error_response_str(&dnf_command,
                    action));
            }
        }

        return ActionResult::Success;
    }

    fn systemctrl(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::systemctrl(self, connection, action);
    }

    fn firewall(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // fedora apparently needs ufw firewall enabled first before adding rules, despite the
        // man page saying it's supported, and it working on debian/ubuntu
        return common_actions_linux::firewall(self, connection, action, true);
    }

    fn edit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::edit_file(self, connection, action);
    }

    fn copy_path(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::copy_path(self, connection, action);
    }

    fn remove_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::remove_file(self, connection, action);
    }

    fn download_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::download_file(self, connection, action);
    }

    fn transmit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::transmit_file(self, connection, action);
    }

    fn receive_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::receive_file(self, connection, action);
    }

    fn create_symlink(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::create_symlink(self, connection, action);
    }

    fn set_time_zone(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::set_time_zone(self, connection, action);
    }

    fn disable_swap(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::disable_swap(self, connection, action);
    }

    fn create_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::create_file(self, connection, action);
    }

    fn add_group(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::add_group(self, connection, action);
    }

    fn set_hostname(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::set_hostname(self, connection, action);
    }

    fn create_systemd_service(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::create_systemd_service(self, connection, action);
    }
}
