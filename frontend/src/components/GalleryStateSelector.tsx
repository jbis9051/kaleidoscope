import styles from "@/components/GalleryStateSelector.module.css";
import {timestampToDateShort} from "@/utility/mediaMetadata";
import {QueryState} from "@/hooks/useQueryState";
import {Media} from "@/api/api";

export interface GalleryStateSelectorProps {
    galleryState: QueryState;
    setGalleryState: (newState: Partial<QueryState>) => void;

    oldest: Media | null;
    newest: Media | null;
    count: number;

    size: number;
    setSize: (size: number) => void;

    removeEnabled: boolean;
    onRemove: () => void;
}

export default function GalleryStateSelector({galleryState, setGalleryState, oldest, newest, count, size, setSize, removeEnabled,  onRemove}: GalleryStateSelectorProps) {
    return (
        <div className={styles.container}>
            <div className={styles.pageSelector}>
                <button
                    disabled={galleryState.page <= 0}
                    onClick={() => setGalleryState({page: Math.max(galleryState.page - 1, 0)})}>
                    -
                </button>
                <span>{galleryState.page + 1}</span>
                <button
                    disabled={galleryState.limit * (galleryState.page + 1) >= count}
                    onClick={() => setGalleryState({page: galleryState.page + 1})}>
                    +
                </button>
            </div>
            <div className={styles.thumbsizeRange}>
                <input type="range" min="50" max="500" value={size}
                       onChange={(e) => setSize(parseInt(e.target.value))}/>
            </div>
            <div className={styles.limitSelector}>
                <select value={galleryState.limit}
                        onChange={(e) => setGalleryState({limit: parseInt(e.target.value, 10)})}>
                    <option value="10">10</option>
                    <option value="20">20</option>
                    <option value="50">50</option>
                    <option value="100">100</option>
                </select>
            </div>
            <div className={styles.orderSelector}>
                <select value={galleryState.orderby} onChange={(e) => setGalleryState({orderby: e.target.value})}>
                    <option value="id">ID</option>
                    <option value="created_at">Created At</option>
                    <option value="size">Size</option>
                    <option value="name">Name</option>
                    <option value="uuid">UUID</option>
                    <option value={"added_at"}>Added At</option>
                </select>
                <button
                    onClick={() => setGalleryState({asc: !galleryState.asc})}>{galleryState.asc ? 'ASC' : 'DESC'}</button>
                <button disabled={!removeEnabled} onClick={onRemove}>Remove
                </button>
            </div>
            <div className={styles.dateRange}>
                <span>{oldest && timestampToDateShort(oldest.created_at)}</span>-
                <span>{newest && timestampToDateShort(newest.created_at)}</span>
            </div>
        </div>
    );
}