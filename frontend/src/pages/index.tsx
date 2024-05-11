import styles from "./index.module.css";
import {useEffect, useState} from "react";
import {API_URL} from "@/global";
import {Api, Media, MediaQueryColumns} from "@/api/api";
import Gallery from "@/components/Gallery";

export default function Index() {

    const [photos, setPhotos] = useState<Media[] | null>(null);

    const [page, setPage] = useState(1);
    const [orderby, setOrderby] = useState<MediaQueryColumns>('id');
    const [asc, setAsc] = useState(true);
    const [limit, setLimit] = useState(10);

    const [size, setSize] = useState(200);

    const [preview, setPreview] = useState<Media | null>(null);

    useEffect(() => {
        const api = new Api(API_URL);
        api.getMedia(page, limit, orderby, asc).then((photos) => {
            setPhotos(photos)
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
                    <img src={`http://localhost:3001/media/${preview.uuid}/full`}/>
                    <div className={styles.previewInfo}>
                        <div>{preview.name}</div>
                        <div>{preview.size} bytes</div>
                        <div>{preview.created_at}</div>
                    </div>
                    <button onClick={() => setPreview(null)}>Close</button>
                </div>
            </div>}
            <div className={styles.statusBar}>
                <span className={styles.title}>Kaleidoscope</span>
            </div>
            <div className={styles.mainFrame}>
                <div className={styles.leftPanel}></div>
                <div className={styles.mainSection}>
                    <div className={styles.mainSectionHeader}>
                        <div className={styles.pageSelector}>
                            <button onClick={() => setPage(page => Math.max(page - 1, 1))}>Prev</button>
                            <span>{page}</span>
                            <button onClick={() => setPage(page => page + 1)}>Next</button>
                        </div>
                        <div className={styles.thumbsizeRange}>
                            <input type="range" min="50" max="500" step={20} value={size}
                                   onChange={(e) => setSize(parseInt(e.target.value))}/>
                            <span>{size}</span>
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
                        {photos && <Gallery media={photos} size={size} open={media => setPreview(media)}/> || <span>Loading...</span>}
                    </div>
                </div>
            </div>
        </div>
    );
}
