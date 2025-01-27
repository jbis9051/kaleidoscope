import styles from "@/components/AlbumSelector.module.css";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faFilter, faFolder} from "@fortawesome/free-solid-svg-icons";
import {AlbumIndex, MediaView} from "@/api/api";
import React from "react";

export interface AlbumProps {
    albums: AlbumIndex[];
    mediaViews: MediaView[];
    lastImportId: number | null;
    selectedAlbum: string | null;

    setSelectedAlbum: (album: AlbumIndex | null) => void;
    selectMediaView: (view: MediaView) => void;
    createAlbum: () => void;
    deleteAlbum: () => void;

    onDrop: (e: React.DragEvent<HTMLDivElement>, album: AlbumIndex) => void;

    mediaViewMatchesCurrentURL: (view: MediaView) => boolean;
}

export default function AlbumSelector({
                                          albums,
                                          mediaViews,
                                          lastImportId,
                                          selectedAlbum,
                                          setSelectedAlbum,
                                          selectMediaView,
                                          createAlbum,
                                          deleteAlbum,
                                          onDrop,
                                          mediaViewMatchesCurrentURL
                                      }: AlbumProps) {

    const [albumHover, setAlbumHover] = React.useState<AlbumIndex | null>(null);

    if(lastImportId !== null) {
        mediaViews = [
            {
                id: -1,
                uuid: 'last-import',
                name: 'Last Import',
                view_query: `orderby=created_at&asc=false&limit=100&import_id=${lastImportId}`,
                created_at: 0,
            },
            ...mediaViews
        ]
    }

    return <div className={styles.albumSelector}>
        <div className={styles.albumHeader}>
            <div className={styles.albumTitle}>Albums</div>
            <div className={styles.albumControls}>
                <button onClick={createAlbum}>New</button>
                <button onClick={deleteAlbum} disabled={!selectedAlbum}>Trash</button>
            </div>
        </div>
        <div className={styles.albumContainer}>
            <div className={styles.albums}>
                <div
                    className={`${styles.album} ${(!selectedAlbum && mediaViews?.every(m => !mediaViewMatchesCurrentURL(m))) && styles.selected}`}
                    onClick={() => setSelectedAlbum(null)}>
                    <FontAwesomeIcon className={styles.albumIcon} icon={faFolder}/>
                    All Photos
                </div>
                {albums && albums.map((album) => (
                    <div
                        onDrop={async (e) => {
                            onDrop(e, album);
                            setAlbumHover(null);
                        }}
                        onDragOver={(e) => {
                            e.preventDefault()
                            e.dataTransfer.dropEffect = 'link';
                        }}
                        onDragEnter={() => setAlbumHover(album)}
                        onDragLeave={() => {
                            setAlbumHover(crnt => {
                                if (crnt && crnt.uuid === album.uuid) {
                                    return null;
                                }
                                return crnt;
                            })
                        }}
                        className={`${styles.album} ${selectedAlbum == album.uuid && styles.selected} ${albumHover?.uuid == album.uuid && styles.hover}`}
                        key={album.uuid}
                        onClick={() => setSelectedAlbum(album)}>
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
                                setSelectedAlbum(null);
                                selectMediaView(view);
                            }}>
                            <FontAwesomeIcon className={styles.albumIcon} icon={faFilter}/>
                            {view.name}
                        </div>
                    }
                )}
            </div>
        </div>
    </div>
}