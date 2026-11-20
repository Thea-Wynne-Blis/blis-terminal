use std::borrow::Cow;

use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};

fn shorthand_heading(line: &str) -> Option<usize> {
    let trimmed = line.trim_start_matches(' ');
    let indent = line.len() - trimmed.len();
    let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if indent > 3 || !(1..=6).contains(&hashes) {
        return None;
    }
    let next = trimmed[hashes..].chars().next()?;
    (!next.is_whitespace()).then_some(indent + hashes)
}

pub(super) fn normalize_headings(source: &str) -> Cow<'_, str> {
    if !source.lines().any(|line| shorthand_heading(line).is_some()) {
        return Cow::Borrowed(source);
    }
    let mut protected = Parser::new(source)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock) => Some(range),
            _ => None,
        })
        .peekable();
    let mut output = String::with_capacity(source.len());
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if let Some(insert) = shorthand_heading(line) {
            let position = offset + insert;
            while protected.peek().is_some_and(|range| range.end <= position) {
                protected.next();
            }
            if !protected
                .peek()
                .is_some_and(|range| range.contains(&position))
            {
                output.push_str(&line[..insert]);
                output.push(' ');
                output.push_str(&line[insert..]);
                offset += line.len();
                continue;
            }
        }
        output.push_str(line);
        offset += line.len();
    }
    Cow::Owned(output)
}

pub(super) fn render_markdown(source: &str) -> String {
    let markdown = normalize_headings(source);
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let events = Parser::new_ext(&markdown, options).filter_map(|event| match event {
        Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text)),
        Event::Start(Tag::Link { .. } | Tag::Image { .. })
        | Event::End(TagEnd::Link | TagEnd::Image) => None,
        other => Some(other),
    });
    let mut output = String::new();
    html::push_html(&mut output, events);
    output
}

pub(super) fn plain_text(markdown: &str) -> String {
    let mut output = String::new();
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(text) | Event::Code(text) => output.push_str(&text),
            Event::Start(Tag::Item) => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("- ");
            }
            Event::TaskListMarker(done) => output.push_str(if done { "[x] " } else { "[ ] " }),
            Event::SoftBreak | Event::HardBreak | Event::Rule => output.push('\n'),
            Event::End(
                TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::CodeBlock | TagEnd::Item,
            ) if !output.ends_with('\n') => output.push('\n'),
            _ => {}
        }
    }
    output
}
