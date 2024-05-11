import {Media} from "@/api/api";
import styles from "./Gallery.module.css";

export default function Gallery({media, size, open}: { media: Media[], size: number , open: (media: Media) => void}) {
    return (
        <div className={styles.container}>
            {media.map((m) => (
                <div onClick={() => open(m)} className={styles.imageContainer} style={{width:`${size}px`, height: `${size}px`}} key={m.id}>
                    <img className={styles.image} src={`http://localhost:3001/media/${m.uuid}/thumb`}/>
                </div>
            ))}
        </div>
    );
}