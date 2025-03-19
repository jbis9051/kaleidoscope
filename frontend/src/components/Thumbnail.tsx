import {Media} from "@/api/api";
import {API_URL} from "@/global";
import styles from "./Thumbnail.module.css";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faFileImage} from "@fortawesome/free-solid-svg-icons";

export interface ThumbnailProps extends React.ImgHTMLAttributes<HTMLImageElement> {
    media: Media
    size?: number
}

export default function Thumbnail({media, ...props}: ThumbnailProps) {
    if (media.has_thumbnail) {
        return <img src={`${API_URL}/media/${media.uuid}/thumb`} alt={media.name} {...props}/>
    }

    return (
        <div className={styles.container} style={{fontSize: props.size ? props.size + "px" : "1em"}}>
            <FontAwesomeIcon className={styles.icon}  icon={faFileImage}/>
            <span className={styles.text}>No Image</span>
        </div>
    );
}