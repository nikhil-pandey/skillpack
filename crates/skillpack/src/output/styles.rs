use owo_colors::Style;
use std::io::IsTerminal;

/// Styling configuration based on terminal capabilities
pub(crate) struct Styles {
    use_color: bool,
}

impl Styles {
    pub(crate) fn new(no_color: bool) -> Self {
        let use_color =
            !no_color && std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        Self { use_color }
    }

    pub(crate) fn header(&self) -> Style {
        if self.use_color {
            Style::new().bold()
        } else {
            Style::new()
        }
    }

    pub(crate) fn name(&self) -> Style {
        if self.use_color {
            Style::new().cyan()
        } else {
            Style::new()
        }
    }

    pub(crate) fn path(&self) -> Style {
        if self.use_color {
            Style::new().dimmed()
        } else {
            Style::new()
        }
    }

    pub(crate) fn success(&self) -> Style {
        if self.use_color {
            Style::new().green()
        } else {
            Style::new()
        }
    }

    pub(crate) fn label(&self) -> Style {
        if self.use_color {
            Style::new().dimmed()
        } else {
            Style::new()
        }
    }

    pub(crate) fn tree(&self) -> Style {
        if self.use_color {
            Style::new().dimmed()
        } else {
            Style::new()
        }
    }

    pub(crate) fn count(&self) -> Style {
        if self.use_color {
            Style::new().yellow()
        } else {
            Style::new()
        }
    }
}
