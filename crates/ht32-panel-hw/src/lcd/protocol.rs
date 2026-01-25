//! LCD protocol definitions and encoding.
//!
//! Protocol structure:
//! - Buffer size: 4104 bytes (1 report byte + 8 header bytes + 4096 data bytes)
//! - Signature byte: 0x55
//! - Command bytes: 0xA1 (config), 0xA2 (refresh), 0xA3 (redraw)

/// Total buffer size including report byte.
pub const BUFFER_SIZE: usize = 4105; // 1 report + 8 header + 4096 data

/// Header size (excluding report byte).
pub const HEADER_SIZE: usize = 8;

/// Data payload size.
pub const DATA_SIZE: usize = 4096;

/// Report byte size (HID report ID).
pub const REPORT_SIZE: usize = 1;

/// Protocol signature byte.
pub const LCD_SIGNATURE: u8 = 0x55;

/// Number of chunks for full screen redraw.
pub const CHUNK_COUNT: usize = 27;

/// Size of the final chunk (320 * 170 * 2 = 108800; 108800 % 4096 = 2304).
pub const FINAL_CHUNK_SIZE: usize = 2304;

/// LCD command types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Configuration commands (orientation, heartbeat).
    Config = 0xA1,
    /// Partial screen refresh.
    Refresh = 0xA2,
    /// Full screen redraw.
    Redraw = 0xA3,
}

/// LCD sub-commands for Config command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SubCommand {
    /// Set display orientation.
    Orientation = 0xF1,
    /// Heartbeat with time (keeps device alive).
    SetTime = 0xF2,
}

/// Redraw sub-commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RedrawPhase {
    /// First chunk of redraw.
    Start = 0xF0,
    /// Middle chunks of redraw.
    Continue = 0xF1,
    /// Final chunk of redraw.
    End = 0xF2,
}

/// Builds an orientation command packet.
pub fn build_orientation_packet(portrait: bool) -> [u8; BUFFER_SIZE] {
    let mut buffer = [0u8; BUFFER_SIZE];
    // Skip report byte (index 0)
    buffer[1] = LCD_SIGNATURE;
    buffer[2] = Command::Config as u8;
    buffer[3] = SubCommand::Orientation as u8;
    buffer[4] = if portrait { 0x02 } else { 0x01 };
    buffer
}

/// Builds a heartbeat (set time) command packet.
pub fn build_heartbeat_packet(hours: u8, minutes: u8, seconds: u8) -> [u8; BUFFER_SIZE] {
    let mut buffer = [0u8; BUFFER_SIZE];
    buffer[1] = LCD_SIGNATURE;
    buffer[2] = Command::Config as u8;
    buffer[3] = SubCommand::SetTime as u8;
    buffer[4] = hours;
    buffer[5] = minutes;
    buffer[6] = seconds;
    buffer
}

/// Builds a partial refresh command packet.
pub fn build_refresh_packet(
    x: u16,
    y: u16,
    width: u8,
    height: u8,
    pixel_data: &[u16],
) -> [u8; BUFFER_SIZE] {
    let mut buffer = [0u8; BUFFER_SIZE];
    buffer[1] = LCD_SIGNATURE;
    buffer[2] = Command::Refresh as u8;

    // Position (little-endian)
    buffer[3] = (x & 0xFF) as u8;
    buffer[4] = (x >> 8) as u8;
    buffer[5] = (y & 0xFF) as u8;
    buffer[6] = (y >> 8) as u8;

    // Dimensions
    buffer[7] = width;
    buffer[8] = height;

    // Copy pixel data (big-endian)
    let data_start = REPORT_SIZE + HEADER_SIZE;
    for (i, &pixel) in pixel_data.iter().enumerate() {
        let offset = data_start + i * 2;
        if offset + 1 < BUFFER_SIZE {
            buffer[offset] = (pixel >> 8) as u8;
            buffer[offset + 1] = (pixel & 0xFF) as u8;
        }
    }

    buffer
}

/// Builds a redraw chunk packet.
pub fn build_redraw_chunk(
    chunk_index: usize,
    pixel_data: &[u16],
    offset_in_image: usize,
) -> [u8; BUFFER_SIZE] {
    let mut buffer = [0u8; BUFFER_SIZE];
    buffer[1] = LCD_SIGNATURE;
    buffer[2] = Command::Redraw as u8;

    // Determine phase
    let phase = match chunk_index {
        0 => RedrawPhase::Start,
        i if i == CHUNK_COUNT - 1 => RedrawPhase::End,
        _ => RedrawPhase::Continue,
    };
    buffer[3] = phase as u8;

    // Sequence number (1-based)
    buffer[4] = (chunk_index + 1) as u8;

    // Offset into image (big-endian for offset)
    let byte_offset = offset_in_image * 2;
    buffer[5] = 0; // High byte of offset (unused in original)
    buffer[6] = (byte_offset >> 8) as u8;
    buffer[7] = (byte_offset & 0xFF) as u8;

    // Chunk size
    let chunk_size = if chunk_index == CHUNK_COUNT - 1 {
        FINAL_CHUNK_SIZE
    } else {
        DATA_SIZE
    };
    buffer[8] = (chunk_size >> 8) as u8;
    buffer[9] = (chunk_size & 0xFF) as u8;

    // Copy pixel data (big-endian)
    let pixel_count = chunk_size / 2;
    let data_start = REPORT_SIZE + HEADER_SIZE;
    for i in 0..pixel_count {
        let pixel_idx = offset_in_image + i;
        if pixel_idx < pixel_data.len() {
            let pixel = pixel_data[pixel_idx];
            let offset = data_start + i * 2;
            buffer[offset] = (pixel >> 8) as u8;
            buffer[offset + 1] = (pixel & 0xFF) as u8;
        }
    }

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orientation_packet() {
        let packet = build_orientation_packet(false);
        assert_eq!(packet[1], LCD_SIGNATURE);
        assert_eq!(packet[2], Command::Config as u8);
        assert_eq!(packet[3], SubCommand::Orientation as u8);
        assert_eq!(packet[4], 0x01); // Landscape

        let packet = build_orientation_packet(true);
        assert_eq!(packet[4], 0x02); // Portrait
    }

    #[test]
    fn test_heartbeat_packet() {
        let packet = build_heartbeat_packet(14, 30, 45);
        assert_eq!(packet[1], LCD_SIGNATURE);
        assert_eq!(packet[2], Command::Config as u8);
        assert_eq!(packet[3], SubCommand::SetTime as u8);
        assert_eq!(packet[4], 14);
        assert_eq!(packet[5], 30);
        assert_eq!(packet[6], 45);
    }
}
