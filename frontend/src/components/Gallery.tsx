import {Media} from "@/api/api";
import styles from "./Gallery.module.css";
import {useEffect, useRef, useState} from "react";
import MediaDisplay from "@/components/MediaDisplay";



export default function Gallery({media, size, open, selected, select, setLayout}: {
    media: Media[],
    size: number,
    selected: string[],
    open: (media: Media) => void,
    select: (media: Media | null) => void
    setLayout: (layout: Media[][] | null) => void
}) {

    const [containerWidth, setContainerWidth] = useState(0);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const updateWidth = () => {
            if (containerRef.current) {
                setContainerWidth(containerRef.current.offsetWidth);
            }
        };

        updateWidth();
        window.addEventListener('resize', updateWidth);
        return () => {
            window.removeEventListener('resize', updateWidth);
        };
    }, []);


    const width = size;
    const height = (size * 8) / 7; // 9/16
    const margin = 5 * 2;
    const mediaTotalWidth = width + margin;


    const mediaPerRow = Math.floor(containerWidth / mediaTotalWidth);
    const rows = mediaPerRow > 0 ? Math.ceil(media.length / mediaPerRow): 0;

    const leftOver = containerWidth % mediaTotalWidth;

    const layout = Array.from({length: rows}, (_, i) => media.slice(i * mediaPerRow, (i + 1) * mediaPerRow));


    useEffect(() => {
        setLayout(layout);
    }, [media, mediaPerRow, rows]);

    function Media({m}: { m: Media }) {
        return <div
            onDragStart={(e) => {
                if (!selected.includes(m.uuid)) {
                    select(m);
                    e.dataTransfer.setData('text/json', JSON.stringify({selected: [m.uuid]}));
                } else {
                    e.dataTransfer.setData('text/json', JSON.stringify({selected}));
                }
            }}
            draggable={true}
            onMouseUp={(e) => {
                select(m)
                if(e.detail === 2){
                    open(m);
                }
            }}
            className={`${styles.imageContainer} ${selected.includes(m.uuid) && styles.selected}`}
            style={{width: `${width}px`, height: `${height}px`}} key={m.id}>
            <div className={styles.imageWrapper}>
                <MediaDisplay media={m} preferThumbnail={true} forceThumbnail={true} imgProps={{draggable: false, className: styles.image}} faProps={{className: styles.fa}}/>
            </div>
            <div className={styles.fileName}>{m.name}</div>
        </div>
    }


    return (
        <div className={styles.container} onClick={e => {
            if (e.target === e.currentTarget) {
                select(null);
            }
        }}
             ref={containerRef}
        >
            {layout.map((row, i) => (
                <div className={styles.row} style={{margin: `0 ${leftOver/2}px`}} key={row.map(m => m.id).join('|')} onClick={e => {
                    if (e.target === e.currentTarget) {
                        select(null);
                    }
                }}>
                    {row.map(m => <Media m={m} key={m.uuid}/>)}
                </div>
            ))}
        </div>
    );
}