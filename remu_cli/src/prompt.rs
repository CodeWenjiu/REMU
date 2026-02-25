use std::borrow::Cow;

use reedline::{DefaultPrompt, DefaultPromptSegment, Prompt, PromptEditMode, PromptHistorySearch};

use remu_types::Platform;

#[derive(Clone)]
pub struct RemuPrompt {
    inner: DefaultPrompt,
    multiline_prefix_len: usize,
}

impl RemuPrompt {
    pub fn new(inner: DefaultPrompt, multiline_prefix_len: usize) -> Self {
        Self {
            inner,
            multiline_prefix_len,
        }
    }
}

impl Prompt for RemuPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        self.inner.render_prompt_left()
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        self.inner.render_prompt_right()
    }

    fn render_prompt_indicator(&self, prompt_mode: PromptEditMode) -> Cow<'_, str> {
        self.inner.render_prompt_indicator(prompt_mode)
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        // Align continued lines under the first character after the left prompt.
        Cow::Owned(" ".repeat(self.multiline_prefix_len))
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        self.inner
            .render_prompt_history_search_indicator(history_search)
    }

    fn get_prompt_color(&self) -> reedline::Color {
        self.inner.get_prompt_color()
    }

    fn get_prompt_multiline_color(&self) -> nu_ansi_term::Color {
        self.inner.get_prompt_multiline_color()
    }

    fn get_indicator_color(&self) -> reedline::Color {
        self.inner.get_indicator_color()
    }

    fn get_prompt_right_color(&self) -> reedline::Color {
        self.inner.get_prompt_right_color()
    }

    fn right_prompt_on_last_line(&self) -> bool {
        self.inner.right_prompt_on_last_line()
    }
}

pub fn get_prompt(platform: Platform) -> RemuPrompt {
    let prompt_left = format!("{} ", platform.as_str());
    let prefix_len = prompt_left.len() + 2;
    let inner = DefaultPrompt::new(
        DefaultPromptSegment::Basic(prompt_left.into()),
        DefaultPromptSegment::CurrentDateTime,
    );
    RemuPrompt::new(inner, prefix_len)
}
