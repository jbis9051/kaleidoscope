import React, {useEffect, useRef, useState} from "react";
import Leaflet from 'leaflet';
import 'leaflet/dist/leaflet.css';
import {MapContainer, Marker, Popup, TileLayer, Tooltip, useMap} from "react-leaflet";
import {Media} from "@/api/api";
import MediaImg from "@/components/MediaImg";
import {API_URL} from "@/global";
import styles from "./DynamicMap.module.css";

export interface MapProps extends React.HTMLAttributes<HTMLDivElement> {
    scrollWheelZoom?: boolean,
    center: [number, number],
    zoom: number,
    markers?: [number, number][],
    media?: Media[],
    mediaSize?: number,
    select?: (media: Media) => void
}

function ChangeView({ center, zoom }: { center: [number, number], zoom: number }) {
    const map = useMap();
    map.setView(center, zoom);
    return null;
}

export default function Map({scrollWheelZoom, center, zoom, markers, media, mediaSize, select, ...props}: MapProps) {
    const [centerState, setCenterState] = useState(center);

    useEffect(() => {
        (async function init() {
            // @ts-ignore
            delete Leaflet.Icon.Default.prototype._getIconUrl;
            Leaflet.Icon.Default.mergeOptions({
                iconUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon.png',
                iconRetinaUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon-2x.png',
                shadowUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-shadow.png'
            });
        })();
    }, []);

    let children: React.ReactNode[] = [];

    if (markers) {
        children = children.concat(markers.map((marker, index) => <Marker key={children.length + index} position={marker}/>));
    }

    if (media) {
        const size = mediaSize || 50;
        children = children.concat(media.filter(m => m.latitude && m.longitude).map((media, index) => {
            const icon = new Leaflet.DivIcon({
                className: styles.mediaIcon,
                html: `<img src="${API_URL}/media/${media.uuid}/thumb" >`,
                iconSize: [size, size],
                iconAnchor: [size / 2, size+10]
            });

            return <Marker
                key={children.length + index}
                position={[media.latitude!, media.longitude!]}
                icon={icon}
                eventHandlers={{
                    click: () => {
                        setCenterState([media.latitude!, media.longitude!]);
                        if(select){
                            select(media);
                        }
                    }
                }}
            />;
        }));
    }

    return <div  {...props} >
        <MapContainer
            style={{width: "100%", height: "100%"}}
            center={center}
            zoom={zoom}
            scrollWheelZoom={scrollWheelZoom}
        >
            <>
                <TileLayer
                    url="https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png"
                    attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/">CARTO</a>'
                />
                <ChangeView center={centerState} zoom={zoom}/>
                {...children}
            </>
        </MapContainer>
    </div>;
}