use std::borrow::Cow;

use reedline::{DefaultPrompt, DefaultPromptSegment, Prompt, PromptEditMode, PromptHistorySearch};

pub const PROMPT_LEFT: &str = "remu ";
pub const PROMPT_LEN: usize = PROMPT_LEFT.len() + 2;

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

pub fn get_prompt() -> RemuPrompt {
    let inner = DefaultPrompt::new(
        DefaultPromptSegment::Basic(PROMPT_LEFT.into()),
        DefaultPromptSegment::CurrentDateTime,
    );
    RemuPrompt::new(inner, PROMPT_LEN)
}
