import React, {useEffect, useMemo, useRef, useState} from "react";
import Leaflet from 'leaflet';
import 'leaflet/dist/leaflet.css';
import {MapContainer, Marker, Popup, TileLayer, Tooltip, useMap, useMapEvent} from "react-leaflet";
import {Media} from "@/api/api";
import MediaImg from "@/components/MediaImg";
import {API_URL} from "@/global";
import styles from "./DynamicMap.module.css";
import {number} from "prop-types";
import MarkerClusterGroup from "react-leaflet-cluster";

export interface MapProps extends React.HTMLAttributes<HTMLDivElement> {
    scrollWheelZoom?: boolean,
    center: [number, number],
    zoom: number,
    markers?: [number, number][],
    media?: Media[],
    mediaSize?: number,
    select?: (media: Media) => void,
    centerOnMedia?: boolean
    setRecenterFunction?: (fn: () => void) => void
}

function ChangeView({ center, zoom, centerOnMedia, bounds, setRecenterFunction }: { center: [number, number], zoom: number, centerOnMedia: boolean, bounds: [number, number][] | undefined, setRecenterFunction?: (fn: () => void) => void }) {
    const map = useMap();


    useEffect(() => {
        map.setView(center, zoom);
    }, [center, zoom]);

    function centerOnBounds(){
        if (centerOnMedia && bounds !== undefined) {
            if (bounds.length === 0) {
                map.setView([0, 0], 0);
                return;
            }
            const leaf_bounds = Leaflet.latLngBounds(bounds);
            map.fitBounds(leaf_bounds, { padding: [50, 50] });
        }
    }

    useEffect(() => {
       centerOnBounds();
    }, [bounds, centerOnMedia]);

    useEffect(() => {
        if(setRecenterFunction && centerOnMedia){
            setRecenterFunction(centerOnBounds);
        }
    }, [map, setRecenterFunction, centerOnMedia, bounds, center, zoom]);

    return null;
}

function ZoomListener(){
    const [zoom, setZoom] = useState<number | null>(null);

    useMapEvent('zoomend', (event) => {
        setZoom(event.target.getZoom());
        console.log('Zoom level changed:', event.target.getZoom());
    });

    return null;
};


export default function Map({scrollWheelZoom, center, zoom, markers, media, mediaSize, select, centerOnMedia, setRecenterFunction, ...props}: MapProps) {
    const [centerState, setCenterState] = useState(center);
    const [zoomState, setZoomState] = useState(zoom);

    useEffect(() => {
        setCenterState(center);
        setZoomState(zoom);
    }, [center[0], center[1], zoom]);

    const bounds = useMemo<[number, number][] | undefined>(() => media?.filter(m => m.latitude && m.longitude).map(m => [m.latitude!, m.longitude!] as [number, number]), [media]);

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
        const markers = media.filter(m => m.latitude && m.longitude).map((media, index) => {
            const icon = new Leaflet.DivIcon({
                className: styles.mediaIcon,
                html: media.has_thumbnail ? `<img src="${API_URL}/media/${media.uuid}/thumb" >` : `<span style="color: black; word-break: break-all">${media.name}</span>`,
                iconSize: [size, size],
                iconAnchor: [size / 2, size+10]
            });

            return <Marker
                key={media.uuid}
                position={[media.latitude!, media.longitude!]}
                icon={icon}
                eventHandlers={{
                    click: () => {
                        setCenterState([media.latitude!, media.longitude!]);
                        if(zoomState < 13){
                            setZoomState(13);
                        } else {
                            setZoomState(zoomState + 1);
                        }
                        if(select){
                            select(media);
                        }
                    }
                }}
                // @ts-ignore
                options={{
                    media
                }}
            />;
        });

        if(markers.length > 0){
            children.push(
                <MarkerClusterGroup
                    showCoverageOnHover={false}
                    iconCreateFunction={(cluster: any) => {
                        const count = cluster.getChildCount();
                        const markers = cluster.getAllChildMarkers();

                        return new Leaflet.DivIcon({
                            className: styles.mediaIcon,
                            html: `<span class="${styles.clusterAmount}">${count}</span><img src="${API_URL}/media/${markers[0].options.options.media.uuid}/thumb" >`,
                            iconSize: [size, size],
                            iconAnchor: [size / 2, size+10]
                        });
                    }}
                >
                    {markers}
                </MarkerClusterGroup>
            )
        }
    }

    return <div  {...props} >
        <MapContainer
            style={{width: "100%", height: "100%"}}
            center={center}
            zoom={centerOnMedia ? 0 : zoom}
            scrollWheelZoom={scrollWheelZoom}
        >
            <>
                <TileLayer
                    url="https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png"
                    attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/">CARTO</a>'
                />
                <ChangeView
                    center={centerState}
                    zoom={zoomState}
                    centerOnMedia={centerOnMedia || false}
                    bounds={bounds}
                    setRecenterFunction={setRecenterFunction}
                />
                <ZoomListener/>
                {...children}
            </>
        </MapContainer>
    </div>;
}