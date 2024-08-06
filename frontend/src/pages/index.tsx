import styles from "./index.module.css";
import {useEffect, useState} from "react";
import {API_URL} from "@/global";
import {AlbumIndex, Api, Media, MediaQueryColumns, MediaView} from "@/api/api";
import Gallery from "@/components/Gallery";
import MetadataTable from "@/components/MetadataTable";
import mediaToMetadata from "@/utility/mediaMetadata";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faFilter, faFolder} from "@fortawesome/free-solid-svg-icons";

export default function Index() {
    const [loaded, setLoaded] = useState(false);

    const [photos, setPhotos] = useState<Media[] | null>(null);

    const [page, setPage] = useState(0);
    const [orderby, setOrderby] = useState<MediaQueryColumns>('id');
    const [asc, setAsc] = useState(true);
    const [limit, setLimit] = useState(10);
    const [pathFilter, setPathFilter] = useState<string>("");

    const [count, setCount] = useState(0);


    const [size, setSize] = useState(200);

    const [preview, setPreview] = useState<Media | null>(null);

    const [selected, setSelected] = useState<string | null>(null);

    const [albums, setAlbums] = useState<AlbumIndex[] | null>(null);
    const [selectedAlbum, setSelectedAlbum] = useState<string | null>(null);
    const [albumHover, setAlbumHover] = useState<string | null>(null);

    const [mediaViews, setMediaViews] = useState<MediaView[] | null>(null);

    const api = new Api(API_URL);

    function queryToState() {
        let query = new URLSearchParams(window.location.search);

        const page = query.get('page');
        const orderby = query.get('orderby');
        const asc = query.get('asc');
        const limit = query.get('limit');
        const selectedAlbum = query.get('album');
        const filter_path = query.get('filter_path');

        if (page) {
            setPage(parseInt(page, 10));
        }

        if (orderby) {
            setOrderby(orderby as MediaQueryColumns);
        }

        if (asc) {
            setAsc(asc === 'true');
        }

        if (limit) {
            setLimit(parseInt(limit, 10));
        }

        if (selectedAlbum) {
            setSelectedAlbum(selectedAlbum);
        }

        if (filter_path) {
            setPathFilter(filter_path);
        }

    }

    useEffect(() => {
        queryToState();
        setLoaded(true);
    }, [])

    useEffect(() => {
        function keydown(ev: KeyboardEvent ){
            if(ev.key === "Escape"){
                setSelected(null);
                setPreview(null);
            }

            if(!photos || !selected){
                return;
            }
            if(ev.key === "ArrowRight" || ev.key === "ArrowDown"){
                const selectedIndex = photos.findIndex(photo => photo.uuid === selected);
                if(photos.length > selectedIndex + 1){
                    setSelected(photos[selectedIndex + 1].uuid);
                    if(preview){
                        setPreview(photos[selectedIndex + 1]);
                    }
                }
            }

            if(ev.key === "ArrowLeft" || ev.key === "ArrowUp"){
                const selectedIndex = photos.findIndex(photo => photo.uuid === selected);
                if(selectedIndex - 1 > 0){
                    setSelected(photos[selectedIndex - 1].uuid);
                    if(preview){
                        setPreview(photos[selectedIndex - 1]);
                    }
                }
            }
        }



        window.addEventListener("keydown", keydown);
        return () => {
            window.removeEventListener("keydown", keydown);
        }
    },[selected, photos, preview])

    useEffect(() => {
        if (!loaded) {
            return;
        }
        loadGallery();
    }, [page, orderby, asc, limit, selectedAlbum, loaded]);

    useEffect(() => {
        if (!loaded) {
            return;
        }
        // update the URL
        const query = new URLSearchParams();
        query.set('page', page.toString());
        query.set('orderby', orderby);
        query.set('asc', asc.toString());
        query.set('limit', limit.toString());
        if (selectedAlbum) {
            query.set('album', selectedAlbum);
        }
        if (pathFilter) {
            query.set('filter_path', pathFilter);
        }
        window.history.replaceState({}, '', `${window.location.pathname}?${query.toString()}`);

    }, [page, orderby, asc, limit, selectedAlbum, pathFilter]);


    function loadGallery() {
        let update_promise;
        if (!selectedAlbum) {
            update_promise = api.getMedia(page, limit, orderby, asc, pathFilter).then((photos) => {
                setPhotos(photos.media)
                setCount(photos.count)
            });
        } else {
            update_promise = api.album(selectedAlbum, page, limit, orderby, asc, pathFilter).then((album) => {
                setPhotos(album.media.media)
                setCount(album.media.count)
            });
        }

        update_promise.then(() => {
            if(selected && photos && !photos.some(photo => photo.uuid === selected)){ // if we've selected a photo but it doesn't exist, set it to first
                if(photos.length > 0){
                    setSelected(photos[0].uuid)
                    if(preview){
                        setPreview(photos[0]);
                    }
                } else {
                    setSelected(null)
                    setPreview(null);
                }
            }
        })
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
        if (!selectedAlbum) {
            return;
        }
        const name = albums?.find(a => a.uuid === selectedAlbum)?.name;
        if (confirm(`Are you sure you want to delete ${name}?`)) {
            api.album_delete(selectedAlbum).then(() => {
                setSelectedAlbum(null);
                loadAlbums();
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

    useEffect(() => {
        loadAlbums();
        loadMediaViews();
    }, []);

    function mediaViewMatchesCurrentURL(view: MediaView) {
        const view_query = new URLSearchParams(view.view_query);
        view_query.delete("page");
        view_query.sort();

        const current_query = new URLSearchParams(window.location.search);
        current_query.delete("page");
        current_query.sort();

        return view_query.toString() === current_query.toString();
    }

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
                    <div className={styles.albumSelector}>
                        <div className={styles.albumHeader}>
                            <div className={styles.albumTitle}>Albums</div>
                            <div className={styles.albumControls}>
                                <button onClick={createAlbum}>New</button>
                                <button onClick={deleteAlbum} disabled={!selectedAlbum}>Trash</button>
                            </div>
                        </div>
                        <div className={styles.albumContainer}>
                            <div className={styles.albums}>
                                <div className={`${styles.album} ${(!selectedAlbum && mediaViews?.every(m => !mediaViewMatchesCurrentURL(m)))&& styles.selected}`}
                                     onClick={() => {
                                         if(selectedAlbum || mediaViews?.some(m => mediaViewMatchesCurrentURL(m))){
                                             setPage(0);
                                         }
                                         setSelectedAlbum(null);
                                         setPathFilter("");
                                         loadGallery();
                                     }}>
                                    <FontAwesomeIcon className={styles.albumIcon} icon={faFolder}/>
                                    All Photos
                                </div>
                                {albums && albums.map((album) => (
                                    <div
                                        onDrop={async (e) => {
                                            const dragged = e.dataTransfer.getData('text/json');
                                            const {selected} = JSON.parse(dragged);
                                            await api.album_add_media(album.uuid, selected);
                                            loadAlbums();
                                            setAlbumHover(null);
                                        }}
                                        onDragOver={(e) => {
                                            e.preventDefault()
                                            e.dataTransfer.dropEffect = 'link';
                                        }}
                                        onDragEnter={() => setAlbumHover(album.uuid)}
                                        onDragLeave={() => {
                                            setAlbumHover(crnt => {
                                                if (crnt === album.uuid) {
                                                    return null;
                                                }
                                                return crnt;
                                            })
                                        }}
                                        className={`${styles.album} ${selectedAlbum == album.uuid && styles.selected} ${albumHover == album.uuid && styles.hover}`}
                                        key={album.uuid}
                                        onClick={() => {
                                            setSelectedAlbum(album.uuid);
                                            setSelected(null);
                                        }}>
                                        <FontAwesomeIcon className={styles.albumIcon} icon={faFolder}/>
                                        {album.name} ({album.media_count})
                                    </div>
                                ))}
                                {mediaViews && mediaViews.map((view) => {
                                        const selected = mediaViewMatchesCurrentURL(view);
                                        return <div
                                            className={`${styles.mediaView} ${selected && styles.selected}`}
                                            key={view.uuid}
                                            onClick={() => {
                                                setSelected(null);
                                                setSelectedAlbum(null);
                                                window.history.replaceState({}, '', `${window.location.pathname}?${view.view_query}`);
                                                queryToState();
                                                loadGallery();
                                            }}>
                                            <FontAwesomeIcon className={styles.albumIcon} icon={faFilter}/>
                                            {view.name}
                                        </div>
                                    }
                                )}
                            </div>
                        </div>
                    </div>
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
                                <button onClick={loadGallery}>Filter</button>
                            </div>
                        </div>
                        <div className={styles.filter}>
                            <label>
                                <span>Path </span> <input value={pathFilter} onChange={e => {
                                setPathFilter(e.target.value)
                            }} type="text" placeholder="Path Filter"/>
                            </label>

                        </div>
                    </div>
                </div>
                <div className={styles.mainSection}>
                    <div className={styles.mainSectionHeader}>
                        <div className={styles.pageSelector}>
                            <button disabled={page <= 0} onClick={() => setPage(page => Math.max(page - 1, 0))}>-
                            </button>
                            <span>{page + 1}</span>
                            <button disabled={limit * (page + 1) >= count} onClick={() => setPage(page => page + 1)}>+
                            </button>
                        </div>
                        <div className={styles.thumbsizeRange}>
                            <input type="range" min="50" max="500" value={size}
                                   onChange={(e) => setSize(parseInt(e.target.value))}/>
                        </div>
                        <div className={styles.limitSelector}>
                            <select value={limit} onChange={(e) => setLimit(parseInt(e.target.value, 10))}>
                                <option value="10">10</option>
                                <option value="20">20</option>
                                <option value="50">50</option>
                                <option value="100">100</option>
                            </select>
                        </div>
                        <div className={styles.orderSelector}>
                            <select value={orderby} onChange={(e) => setOrderby(e.target.value as MediaQueryColumns)}>
                                <option value="id">ID</option>
                                <option value="created_at">Created At</option>
                                <option value="size">Size</option>
                                <option value="name">Name</option>
                                <option value="uuid">UUID</option>
                            </select>
                            <button onClick={() => setAsc(asc => !asc)}>{asc ? 'ASC' : 'DESC'}</button>
                            <button disabled={!(selected && selectedAlbum)} onClick={async () => {
                                if (selected && selectedAlbum) {
                                    await api.album_remove_media(selectedAlbum, [selected]);
                                    loadAlbums();
                                    setSelected(null);
                                    loadGallery();
                                }
                            }}>Remove
                            </button>
                        </div>
                    </div>
                    <div className={styles.mainSectionContent}>
                        {photos &&
                            <Gallery
                                media={photos}
                                selected={selected}
                                size={size} open={media => setPreview(media)}
                                select={(m) => setSelected(m.uuid)}
                                clearSelection={() => setSelected(null)}
                            />
                            || <span>Loading...</span>}
                    </div>
                    <div className={styles.mainFooter}>
                        <span>{count} items</span>
                        <span>, {selected ? 1 : 0} selected</span>
                        <span>, Page {page + 1} of {Math.ceil(count / limit)}</span>
                    </div>
                </div>
                <div className={styles.rightPanel}>
                    {(() => {
                            if (!selected || !photos) {
                                return <span>No Photo Selected</span>
                            }
                            const media = photos.find(m => m.uuid === selected);

                            if (!media) {
                                return <span>Selected Photo not found</span>
                            }

                            return <>
                                <div className={styles.previewImageContainer}>
                                    <img draggable={false} src={`${API_URL}/media/${media.uuid}/full`}/>
                                </div>
                                <div className={styles.previewInfoWrapper}>
                                    <div className={styles.previewInfo}>
                                        <MetadataTable metadata={mediaToMetadata(media)}/>
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
