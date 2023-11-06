use async_recursion::async_recursion;

#[derive(Debug)]
pub enum Error {}

use notion;
use notion::BlockType;

pub fn convert_rich_text(text: &notion::RichText) -> String {
    match text {
        notion::RichText::Text {
            text, annotations, ..
        } => {
            let mut string = text.content.to_owned();

            if annotations.bold {
                string = format!("**{string}**");
            }

            if annotations.italic {
                string = format!("*{string}*");
            }

            if annotations.code {
                string = format!("`{string}`");
            }

            string
        }
        _ => "".to_string(),
    }
}

#[async_recursion]
pub async fn convert_blocks(
    notion: &notion::Client,
    blocks: &Vec<notion::Block>,
) -> Result<String, Error> {
    let mut output = vec![];

    for block in blocks.iter() {
        let string = match &block.block {
            BlockType::Heading1 { heading }
            | BlockType::Heading2 { heading }
            | BlockType::Heading3 { heading } => {
                let content = heading
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                let markdown_heading = match &block.block {
                    BlockType::Heading1 { .. } => "#",
                    BlockType::Heading2 { .. } => "##",
                    BlockType::Heading3 { .. } | _ => "###",
                };

                Some(format!("{markdown_heading} {content}"))
            }
            BlockType::Paragraph { paragraph, .. } => Some(
                paragraph
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>(),
            ),
            BlockType::Code { code, .. } => {
                let language = serde_variant::to_variant_name(&code.language).unwrap();
                let content = code
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                Some(format!("```{language}\n{content}\n```"))
            }
            BlockType::BulletedListItem {
                bulleted_list_item, ..
            } => {
                let content = bulleted_list_item
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                // TODO: Recurse down to `children`

                Some(format!("* {content}"))
            }
            BlockType::NumberedListItem {
                numbered_list_item, ..
            } => {
                // TODO: Hold state for numbering
                let content = numbered_list_item
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                // TODO: Recurse down to `children`

                Some(format!("1. {content}"))
            }
            BlockType::ToDo { to_do, .. } => {
                let content = to_do
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                let checked = if to_do.checked.unwrap_or(false) {
                    "x"
                } else {
                    " "
                };

                // TODO: Recurse down to `children`

                Some(format!("[{checked}] {content}"))
            }
            BlockType::Quote { quote, .. } => {
                let content = quote
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                // TODO: Recurse down to `children`

                Some(format!("> {content}"))
            }
            BlockType::Callout { callout, .. } => {
                let content = callout
                    .rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                let icon = if let Some(value) = &callout.icon {
                    match value {
                        notion::Icon::Emoji { emoji, .. } => emoji,
                        _ => "",
                    }
                } else {
                    ""
                };

                // TODO: Recurse down to `children`

                Some(format!("> {icon} {content}"))
            }
            BlockType::Image { image, .. } => {
                match &image {
                    notion::File::External { external, .. } => {
                        let url = &external.url;
                        Some(format!(r#"<img style="margin: 0 auto" src="{url}">"#))
                    }
                    // TODO: Implement reupload of Notion file type
                    _ => None,
                }
            }
            BlockType::Video { video, .. } => {
                match &video {
                    notion::File::External { external, .. } => {
                        let url = &external.url;
                        Some(format!(r#"<video controls src="{url}" />"#))
                    }
                    // TODO: Implement reupload of Notion file type
                    _ => None,
                }
            }
            BlockType::Divider => Some("---".to_string()),
            BlockType::Unsupported => {
                // println!("Did not catch {string}");
                None
            }
            BlockType::ColumnList { .. } => {
                if block.has_children {
                    let columns = notion
                        .blocks
                        .children()
                        .list(notion::BlockChildrenListOptions {
                            block_id: &block.id,
                        })
                        .await
                        .unwrap()
                        .results;

                    let mut content = vec![];
                    for column in columns.iter() {
                        let children = notion
                            .blocks
                            .children()
                            .list(notion::BlockChildrenListOptions {
                                block_id: &column.id,
                            })
                            .await
                            .unwrap()
                            .results;

                        content.push(convert_blocks(&notion, &children).await.unwrap());
                    }

                    Some(format!(
                        r#"<div style="display: flex;">{content}</div>"#,
                        content = content
                            .iter()
                            .map(|column| format!(r#"<div style="margin: 0 16px">{column}</div>"#))
                            .collect::<Vec<String>>()
                            .join("\n")
                    ))
                } else {
                    None
                }
            }

            BlockType::Column { .. }
            | BlockType::Table
            | BlockType::Bookmark { .. }
            | BlockType::File { .. }
            | BlockType::Pdf { .. }
            | BlockType::TableOfContents
            | BlockType::ChildPage { .. }
            | BlockType::ChildDatabase { .. }
            | BlockType::SyncedBlock
            | BlockType::Template
            | BlockType::Toggle
            | BlockType::Breadcrumb
            | BlockType::Embed { .. }
            | BlockType::Equation { .. }
            | BlockType::LinkPreview { .. }
            | BlockType::TableRow
            | BlockType::LinkToPage { .. } => None,
        };

        if let Some(string) = string {
            output.push(string);
        }
    }

    Ok(output.join("\n\n"))
}
