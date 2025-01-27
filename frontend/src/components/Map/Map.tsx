import dynamic from 'next/dynamic';
import {MapProps} from "@/components/Map/DynamicMap";

const DynamicMap = dynamic(() => import('./DynamicMap'), {
    ssr: false
});

export default function Map({...props}: MapProps) {
    return <DynamicMap {...props} />;
}

