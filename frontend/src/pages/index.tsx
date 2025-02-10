import styles from "./index.module.css";
import {useEffect, useState} from "react";
import {API_URL} from "@/global";
import {AlbumIndex, Api, Media, MediaQuery, MediaView} from "@/api/api";
import Gallery from "@/components/Gallery";
import MetadataTable from "@/components/MetadataTable";
import mediaToMetadata from "@/utility/mediaMetadata";
import AlbumSelector from "@/components/AlbumSelector";
import {QueryState, useQueryState} from "@/hooks/useQueryState";
import GalleryStateSelector, {ViewType} from "@/components/GalleryStateSelector";
import {useMediaSelector} from "@/hooks/useMediaSelector";
import FileViewer from "@/components/FileViewer";
import MediaImg from "@/components/MediaImg";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faDownload, faFloppyDisk} from "@fortawesome/free-solid-svg-icons";
import Map from "@/components/Map/Map";
import MapViewer from "@/components/MapViewer";
import Filter from "@/utility/Filter";
import FilterPanel from "@/components/FilterPanel";
import Timeline, {getInterval} from "@/components/Timeline";

export interface MediaViewFilter extends MediaView {
    filter: Filter | null;
    album: string | null;
}

interface ViewQuery {
    filter: string;
    album: string | null;
}

export default function Index() {
    const [initialLoaded, setInitialLoaded] = useState(false);

    const initialQuery = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();

    const [viewType, setViewType] = useState<ViewType>(ViewType.Gallery);

    const [galleryState, setGalleryState, queryToState] = useQueryState({
        page: 0,
        orderby: 'created_at',
        asc: true,
        limit: 100,
        selectedAlbum: null,
        filter: Filter.empty()
    });

    const [media, setMedia] = useState<Media[] | null>(null);
    const [albums, setAlbums] = useState<AlbumIndex[] | null>(null);
    const [mediaViews, setMediaViews] = useState<MediaViewFilter[] | null>(null);
    const [lastImportId, setLastImportId] = useState<number | null>(null);

    const [count, setCount] = useState(0);

    const [size, setSize] = useState(200);

    const [preview, setPreview] = useState<Media | null>(null);

    const [layout, setLayout] = useState<Media[][] | null>(null);

    const {
        selected: selectedMedia,
        select: selectMedia,
        target: selectMediaTarget,
    } = useMediaSelector(media || [], layout);

    const api = new Api(API_URL);

    useEffect(() => {
        if (preview && selectMediaTarget?.uuid !== preview.uuid) {
            setPreview(selectMediaTarget);
        }
    }, [selectMediaTarget]);

    // initially load the gallery state from the URL
    useEffect(() => {
        setGalleryState(queryToState(initialQuery));
        loadAlbums();
        loadMediaViews();
        setInitialLoaded(true);
    }, []);

    useEffect(() => {
        let lastEscape = 0

        function onKeyDown(e: KeyboardEvent) {
            if (e.key === 'Escape' && preview) {
                setPreview(null);
                return;
            }
            if (e.key === 'Escape' && selectedMedia.length > 0) {
                selectMedia(null);
                return;
            }
            if (e.key === 'Escape' && !preview && selectedMedia.length === 0) { // clear filter on double escape
                if (Date.now() - lastEscape < 250) {
                    setGalleryState({filter: Filter.empty()})
                }
                lastEscape = Date.now();
            }

            if (e.key === ' ' && e.target === document.body) {
                if (preview) {
                    setPreview(null);
                } else if (selectMediaTarget) {
                    setPreview(selectMediaTarget);
                }
            }
        }

        window.addEventListener('keydown', onKeyDown);
        return () => window.removeEventListener('keydown', onKeyDown);

    }, [preview, selectedMedia.length, selectMediaTarget]);

    // load the gallery when the gallery state changes
    useEffect(() => {
        if (!initialLoaded) {
            return;
        }
        loadGallery();

        if (galleryState.selectedAlbum && viewType === ViewType.FileBrowser) { // browser doesn't support albums
            setViewType(ViewType.Gallery);
            setGalleryState({filter: Filter.empty()});
        }

    }, [galleryState]);

    function stateToMediaQuery(queryState: QueryState): MediaQuery {
        return queryState.filter.toMediaQuery(queryState.orderby, queryState.asc, queryState.limit, queryState.page);
    }

    function loadGallery() {
        if (!galleryState.selectedAlbum) {
            return api.media_index(stateToMediaQuery(galleryState)).then((photos) => {
                setMedia(photos.media)
                setCount(photos.count)
            });
        } else {
            return api.album(galleryState.selectedAlbum, stateToMediaQuery(galleryState)).then((album) => {
                setMedia(album.media.media)
                setCount(album.media.count)
            });
        }
    }

    function loadAlbums() {
        api.album_index().then((albums) => {
            setAlbums(albums);
        });
    }

    function loadMediaViews() {
        api.media_view_index().then((mediaViews) => {
            setLastImportId(mediaViews.last_import_id);
            setMediaViews(mediaViews.media_views.map(mv => {
                try {
                    const view_query: ViewQuery = JSON.parse(mv.view_query);
                    const filter = Filter.fromString(view_query.filter);
                    const album = view_query.album;
                    return {...mv, filter, album};
                } catch (e: any) {
                    console.error(`Error parsing media view ${mv.name}: ${e.message}`);
                    return {...mv, filter: null, album: null};
                }
            }));
        });
    }

    function createAlbum() {
        const name = prompt('Album Name');
        if (albums?.find(a => a.name === name)) {
            alert('Album with that name already exists');
            return;
        }
        if (name) {
            api.album_create(name).then(() => loadAlbums());
        }
    }

    function deleteAlbum() {
        if (!galleryState.selectedAlbum) {
            return;
        }
        const name = albums?.find(a => a.uuid === galleryState.selectedAlbum)?.name;
        if (confirm(`Are you sure you want to delete ${name}?`)) {
            api.album_delete(galleryState.selectedAlbum).then(() => {
                setGalleryState({selectedAlbum: null});
                loadAlbums();
            });
        }
    }


    function mediaViewMatchesCurrentURL(view: MediaViewFilter) {
        if (!view.filter) {
            return false;
        }
        return galleryState.filter.equals(view.filter);
    }

    const oldest = media && media.length > 0 && media.reduce((prev, current) => (prev.created_at < current.created_at) ? prev : current) || null;
    const newest = media && media.length > 0 && media.reduce((prev, current) => (prev.created_at > current.created_at) ? prev : current) || null;

    return (
        <div className={styles.topLevel}>
            {preview && <div className={styles.preview} onClick={e => {
                if (e.target === e.currentTarget) {
                    setPreview(null);
                }
            }}>
                <div className={styles.previewWrapper}>
                    <MediaImg media={preview}/>
                    <button onClick={() => setPreview(null)}>X</button>
                </div>
            </div>}
            <div className={styles.statusBar}>
                <span className={styles.title}>Kaleidoscope</span>
            </div>
            <div className={styles.mainFrame}>
                <div className={styles.leftPanel}>
                    <AlbumSelector
                        albums={albums || []}
                        mediaViews={mediaViews || []}
                        lastImportId={lastImportId}
                        selectedAlbum={galleryState.selectedAlbum}
                        setSelectedAlbum={(album) => {
                            if (!album) {
                                let filter = Filter.empty();
                                if (viewType === ViewType.MapViewer) {
                                    const gps = galleryState.filter.get('has_gps', "=");
                                    filter = filter.set('has_gps', "=", gps);
                                }
                                setGalleryState({
                                    selectedAlbum: null,
                                    page: 0,
                                    filter,
                                });
                                return;
                            }
                            setGalleryState({selectedAlbum: album.uuid});
                        }}
                        selectMediaView={(view) => {
                            if (!view.filter) {
                                if (confirm(`Invalid filter '${view.name}', delete?`)) {
                                    api.media_view_delete(view.uuid).then(() => loadMediaViews());
                                }
                                return;
                            }
                            setGalleryState({filter: view.filter.clone(), selectedAlbum: view.album});
                        }}
                        createAlbum={createAlbum}
                        deleteAlbum={deleteAlbum}
                        onDrop={async (e, album) => {
                            const dragged = e.dataTransfer.getData('text/json');
                            const {selected} = JSON.parse(dragged);
                            await api.album_add_media(album.uuid, selected);
                            loadAlbums();
                        }}
                        mediaViewMatchesCurrentURL={mediaViewMatchesCurrentURL}/>
                </div>
                <div className={styles.mainSection}>
                    <div className={styles.mainTop}>
                        <GalleryStateSelector
                            galleryState={galleryState}
                            setGalleryState={setGalleryState}
                            oldest={oldest}
                            newest={newest}
                            count={count}
                            size={size}
                            setSize={setSize}
                            viewType={viewType}
                            setViewType={setViewType}
                            removeEnabled={!!(selectedMedia.length > 0 && galleryState.selectedAlbum)}
                            onRemove={async () => {
                                if (selectedMedia.length > 0 && galleryState.selectedAlbum) {
                                    await api.album_remove_media(galleryState.selectedAlbum, selectedMedia.map(m => m.uuid));
                                    loadGallery();
                                    loadAlbums();
                                    selectMedia(null);
                                }
                            }}/>
                        <FilterPanel
                            api={api}
                            filter={galleryState.filter}
                            trashEnabled={!!mediaViews?.some(m => mediaViewMatchesCurrentURL(m))}
                            setFilter={(filter) => setGalleryState({filter: filter.clone()})}
                            onTrash={async () => {
                                if (!mediaViews) {
                                    return;
                                }
                                const selected = mediaViews.find(m => mediaViewMatchesCurrentURL(m));
                                if (!selected) {
                                    return;
                                }
                                if (confirm(`Are you sure you want to delete ${selected.name}?`)) {
                                    await api.media_view_delete(selected.uuid).then(() => {
                                        loadMediaViews();
                                    });
                                }
                            }}
                            onSave={async () => {
                                const name = prompt('Filter Name');
                                if (name) {
                                    const viewQuery = JSON.stringify({
                                        filter: galleryState.filter.toFilterString(),
                                        album: galleryState.selectedAlbum
                                    });
                                    await api.media_view_create(name, viewQuery).then(() => {
                                        loadMediaViews()
                                    });
                                }
                            }}
                        />
                        {initialLoaded &&
                        <Timeline
                            filter={galleryState.filter}
                            selectedAlbum={galleryState.selectedAlbum}
                            api={api}
                            setGalleryState={setGalleryState}
                            mediaRange={ oldest && newest ? [oldest.created_at, newest.created_at] : null}
                        />}
                    </div>
                    <div className={styles.mainSectionContent}>
                        {initialLoaded && viewType === ViewType.FileBrowser &&
                            <FileViewer
                                api={api}
                                filter={galleryState.filter}
                                setGalleryState={setGalleryState}
                                media={media}
                                open={media => setPreview(media)}
                                selected={selectedMedia.map(m => m.uuid)}
                                select={selectMedia}
                                setLayout={setLayout}
                                setViewType={setViewType}
                            />
                        }
                        {initialLoaded && viewType === ViewType.MapViewer &&
                            <MapViewer
                                media={media}
                                select={selectMedia}
                                filter={galleryState.filter}
                                setGalleryState={setGalleryState}
                            />
                        }
                        {viewType === ViewType.Gallery && (media &&
                            <Gallery
                                media={media}
                                size={size}
                                open={media => setPreview(media)}
                                selected={selectedMedia.map(m => m.uuid)}
                                select={selectMedia}
                                setLayout={setLayout}
                            />
                            || <span>Loading...</span>)}
                    </div>
                    <div className={styles.mainFooter}>
                        <span>{count} items</span>
                        <span>, {selectedMedia.length} selected</span>
                        <span>, Page {galleryState.page + 1} of {Math.ceil(count / galleryState.limit)}</span>
                    </div>
                </div>
                <div className={styles.rightPanel}>
                    {(() => {
                            if (!selectMediaTarget || !media) {
                                return <span>No Photo Selected</span>
                            }
                            const m = media.find(m => m.uuid === selectMediaTarget.uuid);

                            if (!m) {
                                return <span>Selected Photo not found</span>
                            }

                            // @ts-ignore
                            return <>
                                <div className={styles.previewImageContainer}>
                                    <div className={styles.downloadWrapper}>
                                        <FontAwesomeIcon className={styles.downloadButton} icon={faDownload}
                                                         onClick={() => downloadItem(`${API_URL}/media/${m.uuid}/full`, m.name)}/>
                                        <span className={styles.downloadSeparator}/>
                                        <FontAwesomeIcon className={styles.downloadButton} icon={faFloppyDisk}
                                                         onClick={() => downloadItem(`${API_URL}/media/${m.uuid}/raw`, m.name)}/>
                                    </div>
                                    <MediaImg draggable={false} blur={false} media={m}/>
                                </div>
                                <div className={styles.previewInfoWrapper}>
                                    <div className={styles.previewInfo}>
                                        <MetadataTable metadata={mediaToMetadata(m)}/>
                                        {m.latitude && m.longitude &&
                                            <Map
                                                center={[m.latitude, m.longitude]}
                                                zoom={12}
                                                className={styles.previewMap}
                                                scrollWheelZoom={false}
                                                media={[m]}
                                            />}
                                    </div>
                                </div>
                            </>
                        }
                    )()}
                </div>
            </div>
        </div>
    );
}


function downloadItem(url: string, name?: string) {
    const a = document.createElement('a');

    a.href = url;
    a.download = name || url;

    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
}