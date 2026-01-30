use iced::widget::{column, container, row};
use iced::{Element, Length};

/// 网格布局 - 将子元素按指定列数排列
pub fn grid_layout<'a, Message>(
    children: Vec<Element<'a, Message>>,
    columns: usize,
    spacing: u32,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    iced::widget::grid(children.into_iter().collect::<Vec<_>>())
        .spacing(spacing)
        .columns(columns)
        .into()
}

/// 响应式网格布局 - 根据可用宽度自动计算列数（使用 row-based 实现）
pub fn responsive_grid<'a, Message>(
    children: Vec<Element<'a, Message>>,
    min_card_width: f32,
    spacing: u32,
    available_width: f32,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    // 如果没有子元素，返回空容器
    if children.is_empty() {
        return container(column![])
            .width(Length::Fill)
            .into();
    }

    // Calculate optimal column count
    let card_total_width = min_card_width + spacing as f32;
    let columns = (available_width / card_total_width).max(1.0).floor() as usize;

    // 使用 into_iter() 手动分块
    let mut iter = children.into_iter();
    let mut row_elements = Vec::new();

    while let Some(first) = iter.next() {
        // 创建这一行的元素
        let mut row_children = vec![first];

        // 添加剩余的列元素（最多 columns-1 个）
        for _ in 1..columns {
            if let Some(next) = iter.next() {
                row_children.push(next);
            } else {
                break;
            }
        }

        let row_elem = row(row_children)
            .spacing(spacing)
            .width(Length::Fill)
            .into();
        row_elements.push(row_elem);
    }

    column(row_elements)
        .spacing(spacing)
        .width(Length::Fill)
        .into()
}
