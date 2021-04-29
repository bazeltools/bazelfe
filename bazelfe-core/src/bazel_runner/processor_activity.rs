use std::collections::HashMap;

use crate::hydrated_stream_processors::process_bazel_failures::{TargetStory, TargetStoryAction};

pub struct ProcessorActivity {
    pub jvm_segments_indexed: u32,
    pub actions_taken: u32,
    pub target_story_actions: HashMap<String, Vec<TargetStory>>,
}
impl ProcessorActivity {
    pub fn merge(&mut self, o: ProcessorActivity, disable_action_stories_on_success: bool) {
        'target_loop: for (target, story_entries) in o.target_story_actions.into_iter() {
            let mut story_vec = match self.target_story_actions.remove(&target) {
                None => vec![],
                Some(existing) => existing,
            };

            story_vec.extend(story_entries.into_iter());

            let mut last_success_when = None;
            for e in story_vec.iter() {
                if let TargetStoryAction::Success = e.action {
                    if let Some(prev) = last_success_when {
                        if prev < e.when {
                            last_success_when = Some(e.when);
                        }
                    } else {
                        last_success_when = Some(e.when);
                    }
                }
            }
            let updated_vec = if let Some(last_success_when) = last_success_when {
                if disable_action_stories_on_success {
                    break 'target_loop;
                }
                let res_vec: Vec<TargetStory> = story_vec
                    .into_iter()
                    .filter(|e| {
                        if let TargetStoryAction::Success = e.action {
                            e.when >= last_success_when
                        } else {
                            true
                        }
                    })
                    .collect();

                // if all thats left is just a single Success, then nothing to ever reasonably report/noop.
                if res_vec.len() == 1 {
                    Vec::default()
                } else {
                    res_vec
                }
            } else {
                story_vec
            };

            if updated_vec.len() > 0 {
                self.target_story_actions.insert(target, updated_vec);
            }
        }

        self.jvm_segments_indexed += o.jvm_segments_indexed;
        self.actions_taken += o.actions_taken;
    }
}
impl Default for ProcessorActivity {
    fn default() -> Self {
        ProcessorActivity {
            jvm_segments_indexed: 0,
            actions_taken: 0,
            target_story_actions: HashMap::new(),
        }
    }
}
