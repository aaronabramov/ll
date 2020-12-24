use crate::level::Level;
use crate::types;
use std::collections::{BTreeSet, LinkedList};

// Extract tags encoded in the key. we do it by using hashtags.
// e.g. we can add them to the event like this:
// `some.event#dont_print#trace` => wil result in the event called
//  `some.event` and a set of tags ['dont_print', 'trace']
pub(crate) fn extract_tags(string_with_tags: String) -> (String, types::Tags) {
    let key_and_tags = string_with_tags
        .split('#')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if key_and_tags.len() < 2 {
        return (string_with_tags, BTreeSet::new());
    }

    let mut key_and_tags = key_and_tags
        .iter()
        .map(|&s| s.to_string())
        .collect::<LinkedList<String>>();

    if let Some(key) = key_and_tags.pop_front() {
        // If that was the only element we'll make it a key
        if key_and_tags.is_empty() {
            return (key, BTreeSet::new());
        }

        return (key, key_and_tags.into_iter().collect());
    }

    // If something went wrong and we had an empty list after split we'll
    // just return the key as it was given to us
    (string_with_tags, BTreeSet::new())
}

pub(crate) fn extract_log_level_from_tags(tags: &types::Tags) -> Option<Level> {
    let mut result_level = None;

    for tag in tags {
        let found_level = match tag.as_ref() {
            "info" => Some(Level::Info),
            "trace" => Some(Level::Trace),
            "debug" => Some(Level::Debug),
            _ => None,
        };

        if let Some(found_level) = found_level {
            if let Some(entry_level_inner) = result_level {
                // If more than one tag is present, we take the lowest
                result_level = Some(std::cmp::min(found_level, entry_level_inner))
            } else {
                result_level = Some(found_level)
            }
        }
    }

    result_level
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::*;

    #[test]
    fn test_tags_extraction() {
        let mut result = String::new();

        let events = vec![
            "some.event#dont_print",
            "#dont_print",
            "another.event#dont_print#trace#dont_save",
            "#blessed#goals",
            "",
            "event #hey    #another tag #hi",
            "fancy.event#debug",
            "fancy.event#info",
            "many.levels #info #debug #trace",
            "many.levels # #debug #trace",
        ];

        for event in events {
            let (key, tags) = extract_tags(event.to_owned());
            let level = extract_log_level_from_tags(&tags);
            result.push_str(&format!(
                "{:.<45}  {:.<15} => {:.<35} | {:?}\n",
                event,
                key,
                tags.into_iter().collect::<Vec<_>>().join(", "),
                level
            ))
        }

        snapshot!(result, "
some.event#dont_print........................  some.event..... => dont_print......................... | None
#dont_print..................................  #dont_print.... => ................................... | None
another.event#dont_print#trace#dont_save.....  another.event.. => dont_print, dont_save, trace....... | Some(Trace)
#blessed#goals...............................  blessed........ => goals.............................. | None
.............................................  ............... => ................................... | None
event #hey    #another tag #hi...............  event.......... => another tag, hey, hi............... | None
fancy.event#debug............................  fancy.event.... => debug.............................. | Some(Debug)
fancy.event#info.............................  fancy.event.... => info............................... | Some(Info)
many.levels #info #debug #trace..............  many.levels.... => debug, info, trace................. | Some(Info)
many.levels # #debug #trace..................  many.levels.... => debug, trace....................... | Some(Debug)

");
    }
}
