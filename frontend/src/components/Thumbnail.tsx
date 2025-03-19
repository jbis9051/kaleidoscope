import React, {useEffect} from "react";
import {Media} from "@/api/api";
import {API_URL} from "@/global";
import styles from "./Thumbnail.module.css";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faFileImage} from "@fortawesome/free-solid-svg-icons";

export interface ThumbnailProps extends React.ImgHTMLAttributes<HTMLImageElement> {
    media: Media
}

export default function Thumbnail({media, ...props}: ThumbnailProps) {
    if (media.has_thumbnail) {
        return <img src={`${API_URL}/media/${media.uuid}/thumb`} alt={media.name} {...props}/>
    }

    return (
        <div className={styles.container}>
            <div className={styles.iconContainer}>
                <FontAwesomeIcon className={styles.icon}  icon={faFileImage}/>
            </div>
            <div className={styles.textContainer}>
                <svg viewBox="0 0 100 100">
                    <text x={50} y={50} dominantBaseline={"middle"} textAnchor={"middle"}>No Thumbnail</text>
                </svg>
            </div>
        </div>
    );
}