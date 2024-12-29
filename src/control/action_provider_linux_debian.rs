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

use super::control_actions::{ActionProvider, ActionError, ControlAction, GenericError, SystemDetailsResult};
use super::control_common::{ControlSession, ControlSessionParams};

pub struct AProviderLinuxDebian {
    // params which give us some hints as to context of session, i.e. username - sudo vs root, etc.
    session_params: ControlSessionParams,
}

impl AProviderLinuxDebian {
    pub fn new(session_params: ControlSessionParams) -> AProviderLinuxDebian {
        AProviderLinuxDebian { session_params }
    }

    pub fn name() -> String {
        "linux_debian".to_string()
    }
}

impl ActionProvider for AProviderLinuxDebian {
    fn name(&self) -> String {
        AProviderLinuxDebian::name()
    }

    fn get_session_params(&self) -> Option<&ControlSessionParams> {
        Some(&self.session_params)
    }

    fn generic_command(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::generic_command(self, connection, action)
    }

    // this is not really an Action, as it doesn't modify anything, it just returns values, but...
    fn get_system_details(&self, connection: &mut ControlSession) -> Result<SystemDetailsResult, GenericError> {
        common_actions_linux::get_system_details(self, connection)
    }

    fn add_user(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::add_user(self, connection, action)
    }

    fn create_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::create_directory(self, connection, action)
    }

    fn remove_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::remove_directory(self, connection, action)
    }

    fn install_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

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
            return Err(ActionError::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string()));
        }

        if packages_string.is_empty() {
            return Err(ActionError::InvalidParams("The resulting 'packages' string list was empty.".to_string()));
        }

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish", true);
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));

                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }

                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before installing packages...");

                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));

                try_count += 1;
            }
        }

        // unattended-upgr

        // TODO: might be worth polling for locks on /var/lib/dpkg/lock-frontend ?

        // by default, update the list of packages, as with some providers,
        // this needs to be done first, otherwise packages can't be found...
        let update_packages = action.params.get_value_as_bool("update", true);
        if update_packages {
            let apt_get_command = "apt-get -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&apt_get_command));
        }

        // Note: first time around, unless we export this DEBIAN_FRONTEND env variable, we get a
        //       debconf / dpkg-preconfigure issue with $TERM apparently being unset, and complaints
        //       about the frontend not being useable. Interestingly, without setting the env variable,
        //       trying again after the first failure works, and it's not time-dependent...

        let apt_get_command = format!("export DEBIAN_FRONTEND=noninteractive; apt-get -y install {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(
                connection.conn.return_failed_command_error_response_str(&apt_get_command,
                action)));
        }

        Ok(())
    }

    fn remove_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

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
            return Err(ActionError::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string()));
        }

        if packages_string.is_empty() {
            return Err(ActionError::InvalidParams("The resulting 'packages' string list was empty.".to_string()));
        }

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish", true);
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));

                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }

                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before removing packages...");

                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));

                try_count += 1;
            }
        }

        let apt_get_command = format!("export DEBIAN_FRONTEND=noninteractive; apt-get -y remove {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));

        let ignore_failure = action.params.get_value_as_bool("ignoreFailure", false);

        if connection.conn.did_exit_with_error_code() {
            if !ignore_failure {
                return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&apt_get_command,
                    action)));
            }
        }

        Ok(())
    }

    fn systemctrl(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::systemctrl(self, connection, action)
    }

    fn firewall(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        // debian doesn't need ufw firewall enabled first before adding rules
        common_actions_linux::firewall(self, connection, action, false)
    }

    fn edit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::edit_file(self, connection, action)
    }

    fn copy_path(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::copy_path(self, connection, action)
    }

    fn remove_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::remove_file(self, connection, action)
    }

    fn download_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::download_file(self, connection, action)
    }

    fn transmit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::transmit_file(self, connection, action)
    }

    fn receive_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::receive_file(self, connection, action)
    }

    fn create_symlink(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::create_symlink(self, connection, action)
    }

    fn set_time_zone(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::set_time_zone(self, connection, action)
    }

    fn disable_swap(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::disable_swap(self, connection, action)
    }

    fn create_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::create_file(self, connection, action)
    }

    fn add_group(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::add_group(self, connection, action)
    }

    fn set_hostname(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::set_hostname(self, connection, action)
    }

    fn create_systemd_service(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::create_systemd_service(self, connection, action)
    }
}

