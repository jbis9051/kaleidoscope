export interface Media {
    id: number;
    uuid: string;
    name: string;
    created_at: number,
    width: number;
    height: number;
    size: number;
    path: string;
    liked: boolean;
    is_photo: boolean;
    added_at: number;
    duration: number | null;
}

export interface Album {
    id: number;
    uuid: string;
    name: string;
    created_at: number;
    medias: Media[];
}

export type MediaQueryColumns = 'id' | 'uuid' | 'name' | 'created_at' | 'width' | 'height' | 'size' | 'path' | 'liked' | 'is_photo' | 'added_at';

export interface MediaIndexResponse {
    media: Media[];
    count: number;
}

export class Api {
    url: string;
    constructor(url: string) {
        this.url = url;
    }

    getMedia(page: number, limit: number, order_by: MediaQueryColumns, asc: boolean): Promise<MediaIndexResponse> {
        return fetch(`${this.url}/media?page=${page}&limit=${limit}&order_by=${order_by}&asc=${asc}`).then(response => response.json())
    }
}