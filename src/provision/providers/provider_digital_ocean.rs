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

use ureq;
use ureq::Error;
use serde_json::{Value};
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::provision::provision_provider::{ProvisionProvider};
use crate::provision::provision_common::{ProvisionActionType, ProvisionActionResult, ActionResultValues};
use crate::provision::provision_manager::{ListType};
use crate::provision::provision_params::{ProvisionParams};

use crate::column_list_printer::{ColumnListPrinter, Alignment};

#[derive(Serialize, Deserialize)]
struct SizeResultItem {
    slug: String,
    memory: u32,
    vcpus: u32,
    disk: u32,
    transfer: f32,
    description: String,

    price_monthly: f32,
    price_hourly: f32,
}

#[derive(Serialize, Deserialize)]
struct SizeListResults {
    sizes: Vec<SizeResultItem>
}

#[derive(Serialize, Deserialize)]
struct ImageResultItem {
    id: u64,
    name: String,
    distribution: String,
    slug: String,
    public: bool,
    min_disk_size: u32,

    #[serde(rename = "type")]
    ttype: String,

    size_gigabytes: f32,
    description: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct ImageListResults {
    images: Vec<ImageResultItem>
}

#[derive(Serialize, Deserialize)]
struct RegionResultItem {
    name: String,
    slug: String,
    available: bool,
}

#[derive(Serialize, Deserialize)]
struct RegionListResults {
    regions: Vec<RegionResultItem>
}

pub struct ProviderDigitalOcean {
    digital_ocean_api_token: String,
}

impl ProviderDigitalOcean {
    pub fn new() -> ProviderDigitalOcean {
        ProviderDigitalOcean { digital_ocean_api_token: String::new() }
    }
}

impl ProvisionProvider for ProviderDigitalOcean {
    fn name(&self) -> String {
        return "digital_ocean".to_string();
    }

    fn supports_interactive(&self) -> bool {
        return true;
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();
        items.push(("API_TOKEN".to_string(), "API Token to use Digital Ocean API".to_string()));
        return items;
    }

    fn configure_interactive(&mut self) -> bool {
        return false;
    }

    fn configure(&mut self) -> bool {
        let digital_ocean_api_token_env = std::env::var("PROD_DIGITAL_OCEAN_API_TOKEN");
        match digital_ocean_api_token_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $PROD_DIGITAL_OCEAN_API_TOKEN not set correctly.");
                return false;
            }
            Ok(v) => {
                self.digital_ocean_api_token = v.trim().to_string();
                return true;
            }
        }
    }

    fn is_configured(&self) -> bool {
        return !self.digital_ocean_api_token.is_empty();
    }

    // actual commands

    fn list_available(&self, list_type: ListType) -> bool {
        let url = match list_type {
            ListType::Plans => "sizes",
            ListType::Regions => "regions",
            ListType::OSs => "images",
            _ => {
                return false;
            }
        };

        // Note: DigitalOcean requires an API token header even for GET requests
        //       to list things, which is a bit annoying...
        if self.digital_ocean_api_token.is_empty() {
            eprintln!("Digital Ocean requires an API token to be used for list API requests. Please set $PROD_DIGITAL_OCEAN_API_TOKEN.");
            return false;
        }

        let resp = ureq::get(&format!("https://api.digitalocean.com/v2/{}", url))
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .call();
        
        if resp.is_err() {
            eprintln!("Error querying api.digitalocean.com for list request: {:?}", resp.err());
            return false;
        }

        let resp_string = resp.unwrap().into_string().unwrap();

        if list_type == ListType::Regions {
            let results: RegionListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} regions:", results.regions.len());

            let mut clp = ColumnListPrinter::new(3)
                .add_titles(["id", "name", "available"]);

            for region in &results.regions {
                clp.add_row_strings(&[&region.slug, &region.name, if region.available {"true"} else {"false"}]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::Plans {
            let results: SizeListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} plans:", results.sizes.len());

            let mut clp = ColumnListPrinter::new(7)
                .set_alignment_multiple(&vec![2usize, 3, 4, 5, 6], Alignment::Right)
                .add_titles(["id", "desc", "cpus", "memory", "disk", "transfer", "price"]);

            for size in &results.sizes {
                clp.add_row_strings(&[&size.slug, &size.description, &format!("{}", size.vcpus), &format!("{} MB", size.memory),
                                         &format!("{} GB", size.disk), &format!("{} TB", size.transfer), &format!("${}", size.price_monthly)]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::OSs {
            let results: ImageListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} OS images:", results.images.len());

            let mut clp = ColumnListPrinter::new(4);

            for image in &results.images {
                clp.add_row_strings(&[&format!("{}", image.id), &image.distribution, &image.description, &image.status]);
            }

            print!("{}", clp);
        }
        else {
            println!("{}", resp_string);
        }
        
        return true;
    }

    fn get_required_params_for_action(&self, action: ProvisionActionType) -> BTreeSet<&str> {
        let mut params = BTreeSet::new();
        if action == ProvisionActionType::CreateInstance {
            params.insert("name");
            params.insert("region");
            params.insert("size");
            params.insert("image");
        }
        params
    }

    fn create_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let name_str = params.get_value("name", "");
        let region_str = params.get_value("region", "");
        let size_str = params.get_value("size", "");
        let image_str = params.get_value("image", "");
        let ipv6 = params.get_value_as_bool("ipv6", false);

        let resp = ureq::post("https://api.digitalocean.com/v2/droplets")
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .send_json(ureq::json!({
                "name": name_str,
                "region": region_str,
                "size": size_str,
                "image": image_str,
                "ipv6": ipv6,
            }));

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        // TODO: make some of this re-useable for multiple actions...
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        401 => {
                            eprintln!("Error: authentication error with Digital Ocean API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Digital Ocean API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error creating instance0: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error creating instance1: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error creating instance2: ");
                }
            }
            return ProvisionActionResult::Failed("".to_string());
        }
        
        let resp_string = resp.unwrap().into_string().unwrap();
        let parsed_response = serde_json::from_str::<Value>(&resp_string);
        if parsed_response.is_err() {
            eprintln!("Error parsing json response from digitalocean.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        let mut result_values = ActionResultValues::new();

        let parsed_value_map = parsed_response.ok().unwrap();
        eprintln!("Created Digital Ocean droplet instance okay:\n{:?}", parsed_value_map);

        // check it's an array object and other stuff (i.e. check the json is expected)
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
            // we only expect 1 actual instance value...
            let droplet_map = value_as_object.get("droplet");
            if droplet_map.is_none() {
                eprintln!("Error: unexpected json response2 from digitalocean.com: {}", resp_string);
                return ProvisionActionResult::Failed("".to_string());
            }
            let droplet_map = droplet_map.unwrap();
    
            // otherwise, hopefully we have what we need...
//            eprintln!("\nSingular response: {:?}", droplet_map);

            // extract the values we want
            let id_val = droplet_map.get("id");
            match id_val {
                Some(val) => {
                    result_values.values.insert("id".to_string(), val.as_u64().unwrap().to_string());
                },
                _ => {
                    eprintln!("Error: unexpected json response3 from digitalocean.com - missing 'id' param: {}", resp_string);
                    return ProvisionActionResult::Failed("".to_string());
                }
            }
        }
        else {
            eprintln!("Error: unexpected json response1 from digitalocean.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        eprintln!("Digital Ocean droplet created...");
        eprintln!("Waiting for droplet instance to spool up with IP address...");

        // to get hold of the IP address, we need to do an additional API query to the
        // get instance API as it's still in the process of being spooled up..

        let instance_id = result_values.values.get("id").unwrap();

        let max_tries = 5;
        let mut try_count = 0;

        while try_count < max_tries {
            // sleep a bit to give things a chance...
            std::thread::sleep(std::time::Duration::from_secs(15));

            let droplet_info = self.get_value_map_from_get_droplet_call(&instance_id);
            if droplet_info.is_err() {
                return droplet_info.err().unwrap();
            }
            let droplet_info_map = droplet_info.unwrap();

 //           let status_str =  

            // extract the values we want
            let _networks_val = droplet_info_map.get("networks");

            try_count += 1;
        }
        
        return ProvisionActionResult::ActionCreatedInProgress(result_values);
    }
}

impl ProviderDigitalOcean {
    fn get_value_map_from_get_droplet_call(&self, droplet_id: &str) -> Result<serde_json::Value, ProvisionActionResult> {
        let url = format!("https://api.digitalocean.com/v2/droplets/{}", &droplet_id);
        let get_droplet_response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .call();

        if get_droplet_response.is_err() {
            let resp_string = get_droplet_response.unwrap().into_string().unwrap();
            eprintln!("Error parsing json response from digitalocean.com for get droplet call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let resp_string = get_droplet_response.unwrap().into_string().unwrap();
        let parsed_response = serde_json::from_str::<Value>(&resp_string);
        if parsed_response.is_err() {
            eprintln!("Error parsing json response from digitalocean.com for get droplet call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let parsed_value_map = parsed_response.ok().unwrap();
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
            // we only expect 1 actual instance value...
            let droplet_map = value_as_object.get("droplet");
            if droplet_map.is_none() {
                eprintln!("Error: unexpected json response2 from digitalocean.com for get droplet call: {}", resp_string);
                return Err(ProvisionActionResult::Failed("".to_string()));
            }
            let droplet_map = droplet_map.unwrap().clone();

            return Ok(droplet_map);
        }

        return Err(ProvisionActionResult::Failed("".to_string()))
    }
}
