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
    is_photo: boolean;
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
}

export type FormatType = 'standard' | 'heif' | 'video' | 'raw' | 'unknown';

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

export const MediaQueryColumns = ['id', 'uuid', 'name', 'created_at', 'width', 'height', 'size', 'path', 'liked', 'is_photo', 'added_at'];
export type MediaQueryColumnsType = (typeof MediaQueryColumns)[number];


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

export interface MediaQuery {
    order_by?: MediaQueryColumnsType;
    asc?: boolean;
    limit?: number;
    page?: number;
    filter_path?: string;
    filter_not_path?: string;
    before?: Date;
    after?: Date;
    is_screenshot?: boolean;
    import_id?: number;
    has_gps?: boolean;
}


export class Api {
    url: string;
    constructor(url: string) {
        this.url = url;
    }

    private queryToParams(query: MediaQuery): string {
        return `${Object.entries(query).map(([key, value]) => {
            if (value === null || value === undefined) {
                return null;
            }
            if (value instanceof Date) {
                return `${key}=${value.getTime()}`;
            } else {
                return `${key}=${value}`;
            }
        }).filter(x => x !== null).join('&')}`;
    }

    getMedia(mediaQuery: MediaQuery): Promise<MediaIndexResponse> {
        return fetch(`${this.url}/media?${this.queryToParams(mediaQuery)}`).then(response => response.json())
    }

    async album_index(): Promise<AlbumIndex[]> {
        const indexes: [Album, number][] = await fetch(`${this.url}/album`).then(response => response.json());
        return indexes.map(([album, media_count]) => ({...album, media_count}));
    }

    album(uuid: string, mediaQuery: MediaQuery): Promise<AlbumResponse> {
        return fetch(`${this.url}/album/${uuid}?${this.queryToParams(mediaQuery)}`).then(response => response.json())
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
    
}