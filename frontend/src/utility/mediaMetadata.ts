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


export default function mediaToMetadata(media: Media): Record<string, string> {
    return {
        "ID": media.id.toString(),
        "Name": media.name,
        "Created At": timestampToDate(media.created_at),
        "Width": media.width.toString(),
        "Height": media.height.toString(),
        "Size": media.size.toString(),
        "Path": media.path,
        "Liked": media.liked.toString(),
        "Type": media.is_photo ? "Photo" : "Video",
        "Added At": timestampToDate(media.added_at),
        "Duration": media.duration ? media.duration.toString() : "N/A"
    }
}