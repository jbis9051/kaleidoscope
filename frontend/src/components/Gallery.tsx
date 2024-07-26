import {Media} from "@/api/api";
import styles from "./Gallery.module.css";
import {API_URL} from "@/global";

export default function Gallery({media, size, open, selected, select, clearSelection}: {
    media: Media[],
    size: number,
    selected: number | null,
    open: (media: Media) => void,
    select: (media: Media) => void
    clearSelection: () => void
}) {
    const width = size;

    // 9/16
    const height = (size * 8) / 7;
    return (
        <div className={styles.container} onClick={e => {
            if (e.target === e.currentTarget) {
                clearSelection();
            }
        }
        }>
            {media.map((m) => (
                <div onDoubleClick={() => open(m)} onClick={() => select(m)}
                     className={`${styles.imageContainer} ${selected == m.id && styles.selected}`}
                     style={{width: `${width}px`, height: `${height}px`}} key={m.id}>
                    <div className={styles.imageWrapper}>
                        <img className={styles.image} src={`${API_URL}/media/${m.uuid}/thumb`}/>
                    </div>
                    <div className={styles.fileName}>{m.name}</div>
                </div>
            ))}
        </div>
    );
}