pub(super) fn fixed_decimal(value: usize, width: usize) -> String {
    let value = value.to_string();
    if value.len() <= width {
        return format!("{value:0>width$}");
    }
    if width <= 1 {
        return ">".repeat(width);
    }
    format!(">{}", &value[value.len() - (width - 1)..])
}

pub(super) fn format_row_number(row: usize, hexadecimal: bool, offset: usize) -> String {
    let display_row = row.saturating_add(offset);
    if hexadecimal {
        format!("{:02X}", display_row.min(0xff))
    } else {
        format!("{display_row:02}")
    }
}
