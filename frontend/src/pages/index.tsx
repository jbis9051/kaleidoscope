import styles from "./index.module.css";
import {useEffect, useState} from "react";
import {API_URL} from "@/global";
import {Api, Media, MediaQueryColumns} from "@/api/api";
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

    const [selected, setSelected] = useState<number | null>(null);

    useEffect(() => {
        const api = new Api(API_URL);
        api.getMedia(page, limit, orderby, asc).then((photos) => {
            setPhotos(photos.media)
            setCount(photos.count)
        });
    }, [page, orderby, asc, limit]);

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

                    </div>
                    <div className={styles.leftPreview} style={{flex: selected ? 2 : 0}}>
                        {(() => {
                                if (!selected || !photos) {
                                    return <span>No Photo Selected</span>
                                }
                                const media = photos.find(m => m.id === selected);

                                if (!media) {
                                    return <span>Selected Photo not found</span>
                                }

                                return <>
                                    <img src={`${API_URL}/media/${media.uuid}/full`}/>
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
                            <button disabled={page <= 1} onClick={() => setPage(page => Math.max(page - 1, 1))}>-</button>
                            <span>{page}</span>
                            <button disabled={limit*page >= count} onClick={() => setPage(page => page + 1)}>+</button>
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
                        </div>
                    </div>
                    <div className={styles.mainSectionContent}>
                        {photos &&
                            <Gallery
                                media={photos}
                                selected={selected}
                                size={size} open={media => setPreview(media)}
                                select={(m) => setSelected(m.id)}
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
