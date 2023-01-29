
use async_recursion::async_recursion;

#[derive(Debug)]
pub enum Error {
}

use notion;
use notion::BlockType;

pub fn convert_rich_text(text: &notion::RichText) -> String {
    match text {
        notion::RichText::Text(text, _) => {
            let mut string = text.content.to_owned();
            
            if text.annotations.bold {
                string = format!("**{string}**");
            }

            if text.annotations.italic {
                string = format!("*{string}*");
            }

            if text.annotations.code {
                string = format!("`{string}`");
            }

            string
        },
        _ => "".to_string()
    }
}

#[async_recursion]
pub async fn convert_blocks(notion: &notion::Client, blocks: &Vec<notion::Block>) -> Result<String, Error> {
    let mut output = vec![];

    for block in blocks.iter() {
        let string = match &block.value {
            BlockType::Heading1(heading) |
            BlockType::Heading2(heading) |
            BlockType::Heading3(heading) => {
                let content = heading.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                let markdown_heading = match &block.value {
                    BlockType::Heading1(_) => "#",
                    BlockType::Heading2(_) => "##",
                    BlockType::Heading3(_) | _ => "###",
                };

                Some(format!("{markdown_heading} {content}"))
            },
            BlockType::Paragraph(paragraph) => {
                Some(
                    paragraph.rich_text
                        .iter()
                        .map(|text| convert_rich_text(text))
                        .collect::<String>()
                )
            },
            BlockType::Code(code) => {
                let language = serde_variant::to_variant_name(&code.language).unwrap();
                let content = code.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                Some(
                    format!("```{language}\n{content}\n```")
                )
            },
            BlockType::BulletedListItem(list_item) => {
                let content = list_item.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                // TODO: Recurse down to `children`

                Some(
                    format!("* {content}")
                )
            },
            BlockType::NumberedListItem(list_item) => {
                // TODO: Hold state for numbering
                let content = list_item.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                // TODO: Recurse down to `children`

                Some(
                    format!("1. {content}")
                )
            },
            BlockType::ToDo(todo_item) => {
                let content = todo_item.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                let checked = if todo_item.checked.unwrap_or(false) {
                    "x"
                } else {
                    " "
                };

                // TODO: Recurse down to `children`

                Some(
                    format!("[{checked}] {content}")
                )
            },
            BlockType::Quote(quote) => {
                let content = quote.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                // TODO: Recurse down to `children`

                Some(
                    format!("> {content}")
                )
            },
            BlockType::Callout(callout) => {
                let content = callout.rich_text
                    .iter()
                    .map(|text| convert_rich_text(text))
                    .collect::<String>();

                let icon = if let Some(value) = &callout.icon {
                    match value {
                        notion::Icon::Emoji(emoji) => emoji,
                        _ => ""
                    }
                } else {
                    ""
                };

                // TODO: Recurse down to `children`

                Some(
                    format!("> {icon} {content}")
                )
            },
            BlockType::Image(image) => {
                match &image.image {
                    notion::File::External(url) => Some(format!(r#"<img style="margin: 0 auto" src="{url}">"#)),
                    // TODO: Implement reupload of Notion file type
                    _ => None
                }
            },
            BlockType::Video(video) => {
                match &video.video {
                    notion::File::External(url) => Some(format!(r#"<video controls src="{url}" />"#)),
                    // TODO: Implement reupload of Notion file type
                    _ => None
                }
            },
            BlockType::Divider => Some("---".to_string()),
            BlockType::Unsupported(string) => {
                println!("Did not catch {string}");
                None
            },
            BlockType::ColumnList(_) => {
                if block.has_children {
                    let columns = notion
                        .blocks()
                        .children()
                        .list(notion::BlockChildrenListOptions { block_id: &block.id })
                        .await
                        .unwrap()
                        .results;

                    let mut content = vec![];
                    for column in columns.iter() {
                        let children = notion.blocks().children().list(notion::BlockChildrenListOptions { block_id: &column.id })
                            .await
                            .unwrap()
                            .results;

                        content.push(convert_blocks(&notion, &children).await.unwrap());
                    }

                    Some(
                        format!(
                            r#"<div style="display: flex;">{content}</div>"#,
                            content = content
                                .iter()
                                .map(
                                    |column| format!(r#"<div style="margin: 0 16px">{column}</div>"#)
                                )
                                .collect::<Vec<String>>()
                                .join("\n")
                        )
                    )

                } else {
                    None
                }
            },

            BlockType::Column(_) |
            BlockType::Table |
            BlockType::Bookmark |
            BlockType::File(_) | BlockType::PDF(_) |

            BlockType::TableOfContents |
            BlockType::ChildPage(_) |
            BlockType::ChildDatabase(_) |
            BlockType::SyncedBlock |
            BlockType::Template |
            BlockType::Toggle => None
        };

        if let Some(string) = string {
            output.push(string);
        }
    }

    Ok(output.join("\n\n"))
}

