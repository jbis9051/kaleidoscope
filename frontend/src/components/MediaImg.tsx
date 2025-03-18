import React, {useEffect} from "react";
import {Media} from "@/api/api";
import {API_URL} from "@/global";

export interface MediaImgProps extends React.ImgHTMLAttributes<HTMLImageElement> {
    media: Media,
    blur?: boolean,
    loaded?: () => void
}

export default function MediaImg({media, loaded, blur, style, ...props}: MediaImgProps) {
    // this element uses the thumbnail until the full image is loaded
    const [loadedFull, setLoadedFull] = React.useState(false);

    const thumbnail = `${API_URL}/media/${media.uuid}/thumb`;
    const full = `${API_URL}/media/${media.uuid}/full`;

    useEffect(() => {
        setLoadedFull(false);
    }, [media]);

    let thumbnailStyle = blur === false ? {} : {filter: "blur(3px)"};

    return <>
        <img
            src={loadedFull ? full : thumbnail}
            alt={media.name}
            onLoad={() => {
                setLoadedFull(true);
                if (loaded) {
                    loaded();
                }
            }}
            style={{display: loadedFull ? 'block' : 'none', ...style}}
            {...props}
        />
        {!loadedFull &&
            <img
                alt={media.name}
                src={thumbnail}
                style={{...thumbnailStyle, ...style}}
                {...props}
            />
        }
    </>
}