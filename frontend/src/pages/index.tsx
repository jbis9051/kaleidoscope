import styles from "./index.module.css";
import {useEffect, useState} from "react";
import {API_URL} from "@/global";
import {Album, AlbumIndex, Api, Media, MediaQueryColumns} from "@/api/api";
import Gallery from "@/components/Gallery";
import MetadataTable from "@/components/MetadataTable";
import mediaToMetadata from "@/utility/mediaMetadata";

export default function Index() {

    const [photos, setPhotos] = useState<Media[] | null>(null);

    const [page, setPage] = useState(1);
    const [count, setCount] = useState(0);
    const [orderby, setOrderby] = useState<MediaQueryColumns>('id');
    const [asc, setAsc] = useState(true);
    const [limit, setLimit] = useState(10);

    const [size, setSize] = useState(200);

    const [preview, setPreview] = useState<Media | null>(null);

    const [selected, setSelected] = useState<string | null>(null);

    const [albums, setAlbums] = useState<AlbumIndex[] | null>(null);
    const [selectedAlbum, setSelectedAlbum] = useState<string | null>(null);
    const [albumHover, setAlbumHover] = useState<string | null>(null);

    const api = new Api(API_URL);

    useEffect(() => {
        if (!selectedAlbum) {
            api.getMedia(page, limit, orderby, asc).then((photos) => {
                setPhotos(photos.media)
                setCount(photos.count)
            });
        } else {
           loadAlbumPhotos();
        }

    }, [page, orderby, asc, limit, selectedAlbum]);

    function loadAlbumPhotos(){
        if(selectedAlbum){
            api.album(selectedAlbum, page, limit, orderby, asc).then((album) => {
                setPhotos(album.media.media)
                setCount(album.media.count)
            });
        }
    }

    function createAlbum() {
        const name = prompt('Album Name');
        if(albums?.find(a => a.name === name)) {
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

    useEffect(() => {
        loadAlbums();
    }, []);

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
                    <div className={styles.leftTop}>
                        <span>Albums</span>
                        <div className={styles.albumControls}>
                            <button onClick={createAlbum}>New</button>
                            <button onClick={deleteAlbum} disabled={!selectedAlbum}>Trash</button>
                        </div>
                        <div className={styles.albumContainer}>
                            <div className={styles.albums}>
                                <div className={`${styles.album} ${!selectedAlbum && styles.selected}`}
                                     onClick={() => setSelectedAlbum(null)}>All Photos
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
                                        onDragLeave={() => setAlbumHover(null)}
                                        className={`${styles.album} ${selectedAlbum == album.uuid && styles.selected} ${albumHover == album.uuid && styles.hover}`}
                                         key={album.uuid}
                                         onClick={() => {
                                             setSelectedAlbum(album.uuid);
                                             setSelected(null);
                                         }}>{album.name} ({album.media_count})</div>
                                ))}
                            </div>
                        </div>
                    </div>
                    <div className={styles.leftPreview} style={{flex: selected ? 2 : 0}}>
                        {(() => {
                                if (!selected || !photos) {
                                    return <span>No Photo Selected</span>
                                }
                                const media = photos.find(m => m.uuid === selected);

                                if (!media) {
                                    return <span>Selected Photo not found</span>
                                }

                                return <>
                                    <img draggable={false} src={`${API_URL}/media/${media.uuid}/full`}/>
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
                <div className={styles.mainSection}>
                    <div className={styles.mainSectionHeader}>
                        <div className={styles.pageSelector}>
                            <button disabled={page <= 1} onClick={() => setPage(page => Math.max(page - 1, 1))}>-
                            </button>
                            <span>{page}</span>
                            <button disabled={limit * page >= count} onClick={() => setPage(page => page + 1)}>+
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
                                    loadAlbumPhotos();
                                }
                            }}>Remove</button>
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
                        <span>, Page {page} of {Math.ceil(count / limit)}</span>
                    </div>
                </div>
            </div>
        </div>
    );
}
