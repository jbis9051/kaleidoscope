import React, {SVGAttributes, useEffect} from "react";
import {Media, MediaType} from "@/api/api";
import {API_URL} from "@/global";
import {FontAwesomeIcon, FontAwesomeIconProps} from "@fortawesome/react-fontawesome";
import {faFileAudio} from "@fortawesome/free-solid-svg-icons";

export interface MediaImgProps {
    media: Media,
    preferThumbnail: boolean // whether to use the thumbnail or the full image
    forceThumbnail?: boolean
    imgProps?: React.ImgHTMLAttributes<HTMLImageElement>,
    videoProps?: React.VideoHTMLAttributes<HTMLVideoElement>,
    audioProps?: React.AudioHTMLAttributes<HTMLAudioElement>,
    objectProps?: React.ObjectHTMLAttributes<HTMLObjectElement>
    faProps?: Omit<FontAwesomeIconProps, 'icon'>
}

export default function MediaDisplay({media, preferThumbnail, forceThumbnail, imgProps, videoProps, objectProps, faProps, audioProps}: MediaImgProps) {
    // this element uses the thumbnail until the full image is loaded
    const [loadedFull, setLoadedFull] = React.useState(false);

    const thumbnailUrl = `${API_URL}/media/${media.uuid}/thumb`;
    const fullUrl = `${API_URL}/media/${media.uuid}/full`;
    const rawUrl = `${API_URL}/media/${media.uuid}/raw`;

    forceThumbnail ||= false;

    if(forceThumbnail && !preferThumbnail){
        throw new Error("Cannot force thumbnail without preferring thumbnail");
    }

    let useThumbnail = preferThumbnail || media.media_type === MediaType.Photo;

    if(media.media_type !== MediaType.Photo){
        useThumbnail = false;
    }

    useThumbnail ||= forceThumbnail;

    useEffect(() => {
        setLoadedFull(false);
    }, [media]);

    if (useThumbnail) {
        if (media.media_type === MediaType.Audio) {
            return <FontAwesomeIcon icon={faFileAudio} {...faProps}/>
        }
        
        if (!media.has_thumbnail) {
            return <img src={"/missing.svg"} alt={media.name} {...imgProps}/>
        }
        
        if(forceThumbnail){
            return <img
                alt={media.name}
                src={thumbnailUrl}
                {...imgProps}
            />
        }
        return <>
            <img
                src={fullUrl}
                alt={media.name}
                onLoad={() => {
                    setLoadedFull(true)
                }}
                style={{display: loadedFull ? 'block' : 'none'}}
                {...imgProps}
            />
            {!loadedFull &&
                <img
                    alt={media.name}
                    src={thumbnailUrl}
                    {...imgProps}
                />
            }
        </>
    }

    // not a thumbnail

    switch (media.media_type){
        case MediaType.Video:
            return <video
                src={rawUrl}
                controls
                {...videoProps}
            />
        case MediaType.Photo:
            return <img
                src={rawUrl}
                alt={media.name}
                {...imgProps}
            />
        case MediaType.Pdf:
            return <object data={rawUrl} type="application/pdf" {...objectProps}>
                <p>Your browser does not support PDFs. <a href={rawUrl}>Download the PDF</a>.</p>
            </object>
        case MediaType.Audio:
            return <audio
                src={rawUrl}
                controls
                {...audioProps}
            />
        default:
            return <p>Unsupported media type</p>
    }
}