import {Media} from "@/api/api";

export function timestampToDate(timestamp: number) {
    // timestamp is in seconds
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
}

export function timestampToDateShort(timestamp: number) {
    // timestamp is in seconds
    const date = new Date(timestamp * 1000);
    return date.toDateString().split(" ").slice(1).join(" ");
}

export function bytesHumanReadable(bytes: number) {
    const totalBytes = bytes;
    const units = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    while (bytes >= 1024 && i < units.length) {
        bytes /= 1024;
        i++;
    }
    return `${bytes.toFixed(2)} ${units[i]} (${totalBytes} bytes)`;
}

export function GPSFormat(longitude: number, latitude: number) {
    const lat = latitude >= 0 ? "N" : "S";
    const lon = longitude >= 0 ? "E" : "W";
    return `${Math.abs(latitude).toFixed(6)}°${lat} ${Math.abs(longitude).toFixed(6)}°${lon}`;
}

export function durationHumanReadable(milliseconds: number) {
    // HH::MM::SS
    const seconds = Math.floor(milliseconds / 1000);
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const remainingSeconds = seconds % 60;
    return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${remainingSeconds.toString().padStart(2, "0")}`;
}

export function capitalize(s: string) {
    return s.charAt(0).toUpperCase() + s.slice(1);
}


export default function mediaToMetadata(media: Media): Record<string, string> {
    return {
        "ID": media.id.toString(),
        "Name": media.name,
        "Created At": timestampToDate(media.created_at),
        "Width": media.width.toString(),
        "Height": media.height.toString(),
        "Size": bytesHumanReadable(media.size),
        "Path": media.path,
        "Liked": media.liked.toString(),
        "Type": capitalize(media.media_type),
        "Added At": timestampToDate(media.added_at),
        "Duration": media.duration ? durationHumanReadable(media.duration) : "N/A",
        "Screenshot": media.is_screenshot ? "Yes" : "No",
        "GPS": (media.longitude && media.latitude) ? GPSFormat(media.longitude, media.latitude) : "N/A",
    }
}