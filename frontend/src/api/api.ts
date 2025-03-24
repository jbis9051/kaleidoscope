export interface Media {
    // some fields omitted for brevity
    id: number;
    uuid: string;
    name: string;
    created_at: number,
    width: number;
    height: number;
    path: string;
    liked: boolean;
    media_type: MediaType;
    added_at: number;
    duration: number | null;
    hash: string;
    size: number;
    file_created_at: number;
    is_screenshot: boolean
    longitude: number | null;
    latitude: number | null;
    format: FormatType;
    import_id: number;
    has_thumbnail: boolean;
}

export type FormatType = 'standard' | 'heif' | 'video' | 'raw' | 'unknown';

export enum MediaType {
    Photo = 'photo',
    Video = 'video',
    Pdf = 'pdf',
    Other = 'other'
}

export interface Album {
    id: number;
    uuid: string;
    name: string;
    created_at: number;
}

export interface MediaView {
    id: number;
    uuid: string;
    name: string;
    view_query: string;
    created_at: number,
}


export interface MediaIndexResponse {
    media: Media[];
    count: number;
}

export interface MediaViewIndexResponse {
    media_views: MediaView[];
    last_import_id: number;
}

export interface AlbumResponse {
    album: Album;
    media: MediaIndexResponse;
}


export interface AlbumIndex extends Album {
    media_count: number;
}

export interface DirectoryNode {
    name: string;
    items: number;
    children: DirectoryNode[];
}

export interface DirectoryTree {
    root: DirectoryNode;
}

export type MediaQuery = string;

export interface MediaQueryDescription  {
    fields: { [key: string]: string };
    dsl_types: { [key: string]: string[] };
}

export interface Info {
    media_query: MediaQueryDescription;
}

export interface TimelineMonth {
    year: number;
    month: number;
    count: number;
}

export interface TimelineDay extends  TimelineMonth {
    day: number;
}

export interface TimelineHour extends TimelineDay {
    hour: number;
}

export type TimelineInterval = 'month' | 'day' | 'hour';

export type TimelineIntervalData<T extends TimelineInterval> = T extends 'month' ? TimelineMonth : T extends 'day' ? TimelineDay : TimelineHour;
export interface QueueStatusProgress {
    status: 'Progress';
    index: number;
    total: number;
    queue: {
        id: number;
        media_id: number;
        task: string;
        created_at: number;
    };
    error: null | string;
}

export interface QueueStatusEmpty {
    status: 'Empty';
}

export type QueueStatus = QueueStatusProgress | QueueStatusEmpty;

export class Api {
    url: string;
    constructor(url: string) {
        this.url = url;
    }

    media_index(mediaQuery: MediaQuery): Promise<MediaIndexResponse> {
        return fetch(`${this.url}/media?query=${encodeURI(mediaQuery)}`).then(response => response.json())
    }


    media_timeline<T extends TimelineInterval>(mediaQuery: MediaQuery, interval: T): Promise<TimelineIntervalData<T>[]> {
        return fetch(`${this.url}/media/timeline?query=${encodeURI(mediaQuery)}&interval=${interval}`).then(response => response.json())
    }

    async album_index(): Promise<AlbumIndex[]> {
        const indexes: [Album, number][] = await fetch(`${this.url}/album`).then(response => response.json());
        return indexes.map(([album, media_count]) => ({...album, media_count}));
    }

    album(uuid: string, mediaQuery: MediaQuery): Promise<AlbumResponse> {
        return fetch(`${this.url}/album/${uuid}?query=${encodeURI(mediaQuery)}`).then(response => response.json())
    }

    album_timeline<T extends TimelineInterval>(uuid: string, mediaQuery: MediaQuery, interval: T): Promise<TimelineIntervalData<T>[]> {
        return fetch(`${this.url}/album/${uuid}/timeline?query=${encodeURI(mediaQuery)}&interval=${interval}`).then(response => response.json())
    }

    album_create(name: string): Promise<Album> {
        return fetch(`${this.url}/album`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({name})
        }).then(response => response.json())
    }

    album_delete(uuid: string): Promise<void> {
        return fetch(`${this.url}/album/${uuid}`, {
            method: 'DELETE'
        }).then(response => {})
    }

    album_add_media(uuid: string, medias: string[]): Promise<void> {
        return fetch(`${this.url}/album/${uuid}/media`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({medias})
        }).then(response => response.json())
    }

    album_remove_media(uuid: string, medias: string[]): Promise<void> {
        return fetch(`${this.url}/album/${uuid}/media`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({medias})
        }).then(response => response.json())
    }
    
    media_view_index(): Promise<MediaViewIndexResponse> {
        return fetch(`${this.url}/media_view`).then(response => response.json())
    }
    
    media_view_create(name: string, view_query: string): Promise<MediaView> {
        return fetch(`${this.url}/media_view`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({name, view_query})
        }).then(response => response.json())
    }
    
    media_view_delete(uuid: string): Promise<void> {
        return fetch(`${this.url}/media_view`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({uuid})
        }).then(response => {})
    }

    directory_tree(): Promise<DirectoryTree> {
        return fetch(`${this.url}/directory_tree`).then(response => response.json())
    }

    info(): Promise<Info> {
        return fetch(`${this.url}/info`).then(response => response.json())
    }

    queue_status(): Promise<QueueStatus> {
        return fetch(`${this.url}/queue-status`).then(response => response.json())
    }
}