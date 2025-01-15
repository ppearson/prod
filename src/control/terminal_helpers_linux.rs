/*
 Prod
 Copyright 2021-2025 Peter Pearson.
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

// TODO: does this belong in ..HelpersLinux, shouldn't it be in ..HelpersUnix instead?

#[derive(Debug, PartialEq)]
pub struct StatDetails {
    pub file_size:      usize,
    pub access_details: InnerAccessDetails,
}

#[derive(Debug, PartialEq)]
pub struct InnerAccessDetails {
    pub permissions_num:    String,
    pub owner:              String,
    pub group:              String,
}

// returns tuple of access permissions, owner name, group name
pub fn extract_details_from_stat_output(output: &str) -> Option<StatDetails> {

    let mut file_size = None;
    let mut access_details = None;

    let lines = output.lines();
    for line in lines {
        if line.contains("Size: ") && line.len() > 9 {
            // extract the file size
            let size_text_end = line.find("Size: ").unwrap() + 6; // need to go past the space
            if let Some(size_end) = line[size_text_end..].find(char::is_whitespace) {
                let size_str = &line[size_text_end..size_text_end + size_end];
    
                if let Ok(size) = size_str.parse::<usize>() {
                    file_size = Some(size);
                }
            }
        }

        if line.starts_with("Access:") {
            let res = extract_contents_from_brackets(line);
            if let Some(items) = res {
                if items.len() == 3 {
                    let permissions_full = items[0].clone();
                    let owner_full = items[1].clone();
                    let group_full = items[2].clone();

                    let permissions_num = permissions_full.split_once('/').unwrap().0;
                    let permissions_num = permissions_num[1..].to_string();

                    let owner = owner_full.split_once('/').unwrap().1.trim().to_string();
                    let group = group_full.split_once('/').unwrap().1.trim().to_string();

                    access_details = Some(InnerAccessDetails { permissions_num, owner, group });
                }
            }

            // we should be done now...
            break;
        }
    }

    // if we have everything, build the struct and return it
    if let Some(file_size) = file_size {
        if let Some(access_details) = access_details {
            return Some(StatDetails { file_size, access_details });
        }
    }

    None
}

fn extract_contents_from_brackets(string: &str) -> Option<Vec<String>> {
    // make sure we have matched pairs first.
    let count_open = string.matches('(').count();
    let count_close = string.matches(')').count();
    if count_open == 0 || count_open != count_close {
        return None;
    }

    let mut contents = Vec::with_capacity(count_open);

    // just brute-force it for the moment...
    let mut opening_pos = 0;

    for (i, chr) in string.chars().enumerate() {
        if chr == '(' {
            opening_pos = i;
        }
        else if chr == ')' {
            let item_start = opening_pos + 1;
            let item_end = i;
            contents.push(string[item_start..item_end].to_string());
        }
    }

    Some(contents)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_contents1() {
        let res = extract_contents_from_brackets("Access: (0664/-rw-rw-r--)  Uid: ( 1000/   peter)   Gid: ( 1000/   peter)").unwrap();

 //       println!("{}", res[0]);

        assert_eq!(res.len(), 3);
    }

    #[test]
    fn test_extract_stat_details1() {

        let stat_response1 =
r#"  File: 11.tif
  Size: 71231369  	Blocks: 139128     IO Block: 4096   regular file
Device: 10301h/66305d	Inode: 3150685     Links: 1
Access: (0664/-rw-rw-r--)  Uid: ( 1000/   peter)   Gid: ( 1000/   peter)
Access: 2021-10-18 23:26:34.847020215 +1300
Modify: 2021-10-18 23:26:34.827018980 +1300
Change: 2021-10-18 23:26:34.827018980 +1300
 Birth: -
      "#;

//        println!("{}", stat_response1);

        let extracted = extract_details_from_stat_output(&stat_response1);

        let expected_details = StatDetails { file_size: 71231369,
            access_details: InnerAccessDetails {
                permissions_num: "664".to_string(),
                owner: "peter".to_string(),
                group: "peter".to_string() }};

        assert_eq!(extracted, Some(expected_details));
    }
}