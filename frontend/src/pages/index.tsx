import styles from "./index.module.css";
import {useEffect, useRef, useState} from "react";
import {API_URL} from "@/global";
import {AlbumIndex, Api, Media, MediaQueryColumns, MediaView} from "@/api/api";
import Gallery from "@/components/Gallery";
import MetadataTable from "@/components/MetadataTable";
import mediaToMetadata, {timestampToDateShort} from "@/utility/mediaMetadata";
import AlbumSelector from "@/components/AlbumSelector";
import {FilterOps, useQueryState} from "@/hooks/useQueryState";
import GalleryStateSelector from "@/components/GalleryStateSelector";
import {useMediaSelector} from "@/hooks/useMediaSelector";

enum SelectForward {
    Forward,
    Backward
}


type FilterInputOps = {
    [P in keyof FilterOps]: string | null;
};

export default function Index() {
    const [loaded, setLoaded] = useState(false);

    const initialQuery = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();

    const [galleryState, setGalleryState, queryToState] = useQueryState({
        page: 0,
        orderby: 'created_at',
        asc: true,
        limit: 100,
        selectedAlbum: null,
        filter: {path: null, before: null, after: null}
    });


    const [media, setMedia] = useState<Media[] | null>(null);
    const [albums, setAlbums] = useState<AlbumIndex[] | null>(null);
    const [mediaViews, setMediaViews] = useState<MediaView[] | null>(null);


    const [filterInput, setFilterInput] = useState<FilterInputOps>({path: null, before: null, after: null});

    const [count, setCount] = useState(0);

    const [size, setSize] = useState(200);

    const [preview, setPreview] = useState<Media | null>(null);

    const [layout, setLayout] = useState<Media[][]>([]);

    const {selected: selectedMedia, select: selectMedia, target: selectMediaTarget} = useMediaSelector(media || [], layout);

    const api = new Api(API_URL);

    // initially load the gallery state from the URL
    useEffect(() => {
        setGalleryState(queryToState(initialQuery));
        loadAlbums();
        loadMediaViews();
        setLoaded(true);
    }, [])

    // load the gallery when the gallery state changes
    useEffect(() => {
        if (!loaded) {
            return;
        }
        loadGallery();
    }, [galleryState]);

    // update the filterInputs when the filter changes
    useEffect(() => {
        setFilterInput({
            path: galleryState.filter.path,
            before: galleryState.filter.before?.toISOString().split('T')[0] || null,
            after: galleryState.filter.after?.toISOString().split('T')[0] || null
        })
    }, [galleryState.filter])


    function loadGallery() {
        if (!galleryState.selectedAlbum) {
            api.getMedia(galleryState.page, galleryState.limit, galleryState.orderby, galleryState.asc, galleryState.filter.path, galleryState.filter.before, galleryState.filter.after).then((photos) => {
                setMedia(photos.media)
                setCount(photos.count)
            });
        } else {
            api.album(galleryState.selectedAlbum, galleryState.page, galleryState.limit, galleryState.orderby, galleryState.asc, galleryState.filter.path, galleryState.filter.before, galleryState.filter.after).then((album) => {
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
            setMediaViews(mediaViews);
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


    function mediaViewMatchesCurrentURL(view: MediaView) {
        const view_query = new URLSearchParams(view.view_query);
        view_query.delete("page");
        view_query.sort();

        const current_query = new URLSearchParams(window.location.search);
        current_query.delete("page");
        current_query.sort();

        return view_query.toString() === current_query.toString();
    }

    const oldest = media && media.length > 1 && media.reduce((prev, current) => (prev.created_at < current.created_at) ? prev : current) || null;
    const newest = media && media.length > 1 && media.reduce((prev, current) => (prev.created_at > current.created_at) ? prev : current) || null;

    return (
        <div className={styles.topLevel}>
            {preview && <div className={styles.preview} onClick={e => {
                if (e.target === e.currentTarget) {
                    setPreview(null);
                }
            }}>
                <div className={styles.previewWrapper}>
                    <img src={`${API_URL}/media/${preview.uuid}/full`}/>
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
                        selectedAlbum={galleryState.selectedAlbum}
                        setSelectedAlbum={(album) => {
                            if (!album) {
                                setGalleryState({
                                    selectedAlbum: null,
                                    page: 0,
                                    filter: {path: null, before: null, after: null}
                                });
                                return;
                            }
                            setGalleryState({selectedAlbum: album.uuid});
                        }}
                        selectMediaView={(view) => {
                            setGalleryState(queryToState(new URLSearchParams(view.view_query)));
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
                    <div className={styles.filterPanel}>
                        <div className={styles.filterHeader}>
                            <div className={styles.filterTitle}>Filters</div>
                            <div>
                                <button onClick={() => {
                                    const name = prompt('Filter Name');
                                    if (name) {
                                        api.media_view_create(name, window.location.search.substring(1)).then(() => loadMediaViews());
                                    }
                                }}>Save
                                </button>
                                <button
                                    disabled={!mediaViews?.some(m => mediaViewMatchesCurrentURL(m))}
                                    onClick={() => {
                                        if (!mediaViews) {
                                            return;
                                        }
                                        const selected = mediaViews.find(m => mediaViewMatchesCurrentURL(m));
                                        if (!selected) {
                                            return;
                                        }
                                        if (confirm(`Are you sure you want to delete ${selected.name}?`)) {
                                            api.media_view_delete(selected.uuid).then(() => {
                                                loadMediaViews()
                                            });
                                        }
                                    }}>Trash
                                </button>
                                <button onClick={() => {
                                    setGalleryState({
                                        filter: {
                                            path: filterInput.path,
                                            before: filterInput.before ? new Date(filterInput.before) : null,
                                            after: filterInput.after ? new Date(filterInput.after) : null,
                                        }
                                    });
                                }}>Filter
                                </button>
                            </div>
                        </div>
                        <div className={styles.filter}>
                            <label>
                                <span>Path </span> <input value={filterInput.path || ''} onChange={e => {
                                setFilterInput({...filterInput, path: e.target.value})
                            }} type="text" placeholder="Path Filter"/>
                            </label>
                            <label className={styles.filterDate}>
                                <span>Before </span> <input value={filterInput.before || ''}
                                                            onChange={e => {
                                                                setFilterInput({
                                                                    ...filterInput,
                                                                    before: e.target.value
                                                                })
                                                            }} type="date"/>
                            </label>
                            <label className={styles.filterDate}>
                                <span>After </span> <input value={filterInput.after || ''}
                                                           onChange={e => {
                                                               setFilterInput({
                                                                   ...filterInput,
                                                                   after: e.target.value
                                                               })
                                                           }} type="date"/>
                            </label>

                        </div>
                    </div>
                </div>
                <div className={styles.mainSection}>
                    <GalleryStateSelector galleryState={galleryState} setGalleryState={setGalleryState} oldest={oldest}
                                          newest={newest} count={count} size={size} setSize={setSize}
                                          removeEnabled={!!(selectedMedia.length > 0 && galleryState.selectedAlbum)}
                                          onRemove={async () => {
                                              if (selectedMedia.length > 0 && galleryState.selectedAlbum) {
                                                  await api.album_remove_media(galleryState.selectedAlbum, selectedMedia.map(m => m.uuid));
                                                  loadGallery();
                                                  loadAlbums();
                                                  selectMedia(null);
                                              }
                                          }}/>
                    <div className={styles.mainSectionContent}>
                        {media &&
                            <Gallery
                                media={media}
                                selected={selectedMedia.map(m => m.uuid)}
                                size={size} open={media => setPreview(media)}
                                select={selectMedia}
                                setLayout={setLayout}
                            />
                            || <span>Loading...</span>}
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

                            return <>
                                <div className={styles.previewImageContainer}>
                                    <img draggable={false} src={`${API_URL}/media/${m.uuid}/full`}/>
                                </div>
                                <div className={styles.previewInfoWrapper}>
                                    <div className={styles.previewInfo}>
                                        <MetadataTable metadata={mediaToMetadata(m)}/>
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
