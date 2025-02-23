import styles from './Timeline.module.css';
import {Api, TimelineDay, TimelineHour, TimelineInterval, TimelineIntervalData} from "@/api/api";
import {useEffect, useRef, useState} from "react";
import Filter from "@/utility/Filter";
import {QueryState} from "@/hooks/useQueryState";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faMinus, faPlus} from "@fortawesome/free-solid-svg-icons";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

export interface TimelineProps {
    filter: Filter,
    api: Api,
    setGalleryState: (newState: Partial<QueryState>) => void
    mediaRange: [number, number] | null
    selectedAlbum: string | null,
    limit: number,
}

export default function Timeline({
                                     filter,
                                     api,
                                     setGalleryState,
                                     mediaRange,
                                     selectedAlbum,
                                     limit
                                 }: TimelineProps) {
    const timeline = useRef<HTMLDivElement>(null);

    const [data, setData] = useState<TimelineIntervalData<TimelineInterval>[]>([]);
    const [cursor, setCursor] = useState<[number, number] | null>(null);
    const [itemPreview, setItemPreview] = useState<TimelineIntervalData<TimelineInterval> | null>(null);

    const [timeSelection, setTimeSelection] = useState<[number, number] | null>(null);

    const [interval, setInterval] = useState<TimelineInterval>(getInterval(filter));

    useEffect(() => {
        if (selectedAlbum === null) {
            api.media_timeline(filter.toFilterString(), interval).then(data => {
                setData(dataFiller(data, interval, filter));
            });
        } else {
            api.album_timeline(selectedAlbum, filter.toFilterString(), interval).then(data => {
                setData(dataFiller(data, interval, filter));
            });
        }
    }, [interval, filter, api, selectedAlbum]);

    useEffect(() => {
        setInterval(getInterval(filter));
    }, [filter]);

    useEffect(() => {
        function onMouseMove(event: MouseEvent) {
            if (timeline.current) {
                const rect = timeline.current.getBoundingClientRect();
                const x = event.clientX - rect.left;
                const scroll = timeline.current.scrollLeft;
                setCursor([x, scroll]);
            }
        }


        function onMouseLeave() {
            setCursor(null);
            setItemPreview(null);
        }

        function onScroll() {
            if (timeline.current && cursor) {
                const x = cursor[0];
                const scroll = timeline.current.scrollLeft;
                setCursor([x, scroll]);
            }
        }

        if (timeline.current) {
            timeline.current.addEventListener('mousemove', onMouseMove);
            timeline.current.addEventListener('scroll', onScroll)
            timeline.current.addEventListener('mouseleave', onMouseLeave);
        }

        return () => {
            if (timeline.current) {
                timeline.current.removeEventListener('mousemove', onMouseMove);
                timeline.current.removeEventListener('scroll', onScroll);
                timeline.current.removeEventListener('mouseleave', onMouseLeave);
            }
        }

    }, [timeline.current, cursor]);

    const maxCount = data.reduce((max, item) => Math.max(max, item.count), 0);
    const minCount = data.reduce((min, item) => Math.min(min, item.count), Infinity);

    const scale = (count: number) => {
        if (count === -1) {
            return 0;
        }
        if (maxCount === minCount) {
            return 1;
        }
        return (count - minCount) / (maxCount - minCount);
    }


    if (data.length === 0) {
        return <div></div>
    }

    let range = null;

    if (mediaRange !== null) {
        try {
            range = indexOfItemForInterval(interval, data, mediaRange);
        } catch (e) {
            // we are likely in middle of loading data
            //console.error(e);
        }
    }

    const format = (value: number) => value.toString().padStart(2, '0');


    function setFilter(item: TimelineIntervalData<TimelineInterval>, item2?: TimelineIntervalData<TimelineInterval>) {
        const newFilter = filter.clone();


        switch (interval) {
            case "month":
                newFilter.set('created_at', '>=', `${item.year}-${format(item.month)}-01`);
                const end = new Date((item2 || item).year, (item2 || item).month - 1 + 1, 1);
                newFilter.set('created_at', '<', `${end.getUTCFullYear()}-${format(end.getUTCMonth() + 1)}-01`);
                break;
            case "day":
                const dayItem = item as TimelineDay;
                const dayItem2 = item2 as TimelineDay || undefined;
                newFilter.set('created_at', '>=', `${dayItem.year}-${format(dayItem.month)}-${format(dayItem.day)}`);
                const end2 = new Date((dayItem2 || dayItem).year, (dayItem2 || dayItem).month - 1, (dayItem2 || dayItem).day + 1);
                newFilter.set('created_at', '<', `${end2.getUTCFullYear()}-${format(end2.getUTCMonth() + 1)}-${format(end2.getUTCDate())}`);
                break;
            case "hour":
                break;
        }

        setGalleryState({filter: newFilter});
    }

    async function movePage(item: TimelineIntervalData<TimelineInterval>) {
        // we need to calculate the page for the item
        // we begin by create a filter of all medias before the item
        // then we can find the count, and then do some math to find the page

        const outRange = filter.clone();

        switch (interval) {
            case "month":
                outRange.add('created_at', '<', `${item.year}-${format(item.month)}-01`);
                break;
            case "day":
                const dayItem = item as TimelineDay;
                outRange.add('created_at', '<', `${dayItem.year}-${format(dayItem.month)}-${format(dayItem.day)}`);
                break;
            case "hour":
                throw new Error("Not implemented");
        }

        let count = 0;

        if (selectedAlbum === null) {
            await api.media_index(outRange.toFilterString()).then(res => count = res.count);
        } else {
            await api.album(selectedAlbum, outRange.toFilterString()).then(res => count = res.media.count);
        }

        const page = Math.floor(count / limit);

        setGalleryState({page});
    }

    const timeSelectionSorted = timeSelection && [...timeSelection].sort((a, b) => a - b) as [number, number];

    function updateInterval(plus: boolean) {
        const newInterval = intervalChange(interval, plus);
        if (newInterval && safeInterval(filter, newInterval)) {
            setInterval(newInterval);
        }
    }

    return (
        <div className={styles.container}>
            <div className={styles.timelineContainer}>
                {!timeSelectionSorted && <div className={styles.control}>
                    <div className={safeInterval(filter, intervalChange(interval, true)) ? '' : styles.disabled}
                         onClick={() => updateInterval(true)}><FontAwesomeIcon icon={faPlus}/></div>
                    <div className={safeInterval(filter, intervalChange(interval, false)) ? '' : styles.disabled}
                         onClick={() => updateInterval(false)}><FontAwesomeIcon icon={faMinus}/></div>
                </div>}
                <div className={styles.timeline} ref={timeline} onMouseLeave={() => setTimeSelection(null)}>
                    {data.map((item, index) => {
                        let info = null;

                        switch (interval) {
                            case "month":
                                if (item.month === 1) {
                                    info = item.year.toString();
                                }
                                break;
                            case "day":
                                const dayItem = item as TimelineDay;
                                if (dayItem.day === 1) {
                                    info = `${MONTHS[dayItem.month - 1]} ${dayItem.year}`;
                                }
                                break;
                            case "hour":
                                const hourItem = item as TimelineHour;
                                if (hourItem.hour === 0) {
                                    info = `${hourItem.day} ${MONTHS[hourItem.month - 1]} ${hourItem.year}`;
                                }
                                break;
                        }


                        return (
                            <div
                                key={index}
                                className={`${styles.item} ${info && styles.filled}`}
                                data-date={JSON.stringify(item)}
                                onMouseOver={() => {
                                    setItemPreview(item)
                                    setTimeSelection(t => t && [t[0], index] as [number, number]);
                                }}
                                onMouseUp={() => {
                                    if (timeSelectionSorted === null || timeSelectionSorted[0] === timeSelectionSorted[1]) {
                                        movePage(item);
                                    } else {
                                        setFilter(data[timeSelectionSorted[0]], data[timeSelectionSorted[1]]);
                                    }
                                    setTimeSelection(null);
                                }}
                                onMouseDown={() => {
                                    setTimeSelection([index, index]);
                                }}
                            >
                                <div className={styles.barWrapper}>
                                    <div className={styles.bar} style={{height: `${scale(item.count) * 100}%`}}/>
                                </div>
                                <div className={`${styles.infoWrapper}`}>
                                    {info && <div className={styles.info}>{info}</div>}
                                </div>
                            </div>
                        )
                    })}
                    {timeSelectionSorted && (
                        <div className={`${styles.selection} ${styles.highlight}`}
                             style={{
                                 left: `${timeSelectionSorted[0] * 8}px`,
                                 width: `${(timeSelectionSorted[1] - timeSelectionSorted[0] + 1) * 8}px`
                             }}/>
                    )}
                    {range && (
                        <div className={`${styles.range} ${styles.highlight}`}
                             style={{left: `${range[0] * 8}px`, width: `${(range[1] - range[0] + 1) * 8}px`}}/>
                    )}
                    {cursor !== null && (<div className={styles.cursor} style={{left: `${cursor[0] + cursor[1]}px`}}/>)}
                </div>
            </div>
            <div className={styles.status}>
                {itemPreview && <ItemPreview item={itemPreview}/>}
            </div>
        </div>
    )
}

function ItemPreview<T extends TimelineInterval>({item}: { item: TimelineIntervalData<T> }) {
    const {year, month, day, hour} = {day: "1", hour: "12", ...item};

    return (
        <div className={styles.preview}>
            <span className={styles.monthYear}>{MONTHS[month - 1]} {year}</span>
            <span className={styles.day}>{day}</span>
            <span className={styles.hour}>{`${hour}:00`}</span>
            <span className={styles.count}>({Math.max(item.count, 0)})</span>
        </div>
    )
}

function dataFiller<T extends TimelineInterval>(data: TimelineIntervalData<T>[], interval: T, filter: Filter): TimelineIntervalData<T>[] {
    if (data.length === 0) {
        return [];
    }

    const filled: TimelineIntervalData<T>[] = [];
    const map = new Map<string, TimelineIntervalData<T>>();

    const generateKey = (item: TimelineIntervalData<T>): string => {
        switch (interval) {
            case "month":
                return `${item.year}-${item.month}`;
            case "day":
                const dayItem = item as TimelineDay;
                return `${dayItem.year}-${dayItem.month}-${dayItem.day}`;
            case "hour":
                const hourItem = item as TimelineHour;
                return `${hourItem.year}-${hourItem.month}-${hourItem.day}-${hourItem.hour}`;
            default:
                throw new Error("Invalid interval");
        }
    };

    data.forEach(item => map.set(generateKey(item), item));

    let minItem = data[0];
    let maxItem = data[data.length - 1];

    const fillData = (year: number, month: number, day?: number, hour?: number) => {
        const key = [year, month, day, hour].filter(v => v !== undefined).join('-');
        const item = map.get(key);
        if (item) {
            filled.push(item);
        } else {
            const item = {year, month, count: -1} as TimelineIntervalData<T>;
            if (day !== undefined) {
                (item as TimelineDay).day = day;
            }
            if (hour !== undefined) {
                (item as TimelineHour).hour = hour;
            }
            filled.push(item);
        }
    };

    const filterRange = filter.getDateRange('created_at');
    if (filterRange[0] !== null && filterRange[1] !== null) {
        const [start, end] = [new Date(filterRange[0]), new Date(filterRange[1])];
        minItem = {
            year: start.getUTCFullYear(),
            month: start.getUTCMonth() + 1,
            day: start.getUTCDate(),
            hour: start.getUTCHours(),
            count: -1
        } as TimelineIntervalData<T>;
        maxItem = {
            year: end.getUTCFullYear(),
            month: end.getUTCMonth() + 1,
            day: end.getUTCDate(),
            hour: end.getUTCHours(),
            count: -1
        } as TimelineIntervalData<T>;
    }

    for (let year = minItem.year; year <= maxItem.year; year++) {
        const minMonth = minItem.year === maxItem.year && interval !== "month" ? minItem.month : 1;
        const maxMonth = minItem.year === maxItem.year && interval !== "month" ? maxItem.month : 12;

        for (let month = minMonth; month <= maxMonth; month++) {
            if (interval === "month") {
                fillData(year, month);
            } else {
                const minDay = minItem.year === maxItem.year && minItem.month === maxItem.month && interval !== "day" ? (minItem as TimelineDay).day : 1;
                const maxDay = minItem.year === maxItem.year && minItem.month === maxItem.month && interval !== "day" ? (maxItem as TimelineDay).day : 31;
                for (let day = minDay; day <= maxDay; day++) {
                    if (interval === "day") {
                        fillData(year, month, day);
                    } else {
                        for (let hour = 0; hour < 24; hour++) {
                            fillData(year, month, day, hour);
                        }
                    }
                }
            }
        }
    }

    return filled;
}

function indexOfItemForInterval(interval: TimelineInterval, data: TimelineIntervalData<TimelineInterval>[], range: [number, number]): [number, number] {
    const [start, end] = range;
    let [startDate, endDate] = [new Date(start * 1000), new Date(end * 1000)];

    switch (interval) {
        case "month":
            startDate = new Date(Date.UTC(startDate.getUTCFullYear(), startDate.getUTCMonth(), 1));
            endDate = new Date(Date.UTC(endDate.getUTCFullYear(), endDate.getUTCMonth(), 1));
            break;
        case "day":
            startDate = new Date(Date.UTC(startDate.getUTCFullYear(), startDate.getUTCMonth(), startDate.getUTCDate()));
            endDate = new Date(Date.UTC(endDate.getUTCFullYear(), endDate.getUTCMonth(), endDate.getUTCDate()));
            break;
        case "hour":
            startDate = new Date(Date.UTC(startDate.getUTCFullYear(), startDate.getUTCMonth(), startDate.getUTCDate(), startDate.getUTCHours()));
            endDate = new Date(Date.UTC(endDate.getUTCFullYear(), endDate.getUTCMonth(), endDate.getUTCDate(), endDate.getUTCHours()));
            break;
    }

    return indexOfItemForRange(startDate, endDate, data);
}

function indexOfItemForRange(startDate: Date, endDate: Date, data: TimelineIntervalData<TimelineInterval>[]): [number, number] {
    // now we need to find the first and last item that is within the range
    let first = null;
    let last = null;

    // we could do a binary search here, but im lazy
    for (let i = 0; i < data.length; i++) {
        const item = data[i];
        const {year, month, day, hour} = {day: 1, hour: 0, ...item};

        const date = new Date(Date.UTC(year, month - 1, day, hour));

        if (date >= startDate && first === null) {
            first = i;
        }

        if (date <= endDate) {
            last = i;
        }
    }


    if (first === null || last === null) {
        throw new Error("Invalid range");
    }

    return [first, last];
}


export function getInterval(filter: Filter): TimelineInterval {
    const range = filter.getDateRange('created_at');
    const [start, end] = range;

    if (start === null || end === null) {
        return "month";
    }

    const diff = end.getTime() - start.getTime();

    // if we are less than five days, show by hour
    if (diff <= 5 * 24 * 60 * 60 * 1000) {
        return "hour";
    }
    // if we are less than four months, show by day
    if (diff <= 4 * 30 * 24 * 60 * 60 * 1000) {
        return "day";
    }
    // otherwise show by month
    return "month"
}


function intervalChange(current: TimelineInterval, increment: boolean): TimelineInterval | null {
    switch (current) {
        case "month":
            return increment ? "day" : null;
        case "day":
            return increment ? "hour" : "month";
        case "hour":
            return increment ? null : "day";
    }
}

function safeInterval(filter: Filter, interval: TimelineInterval | null): boolean {
    if (interval === null) {
        return false;
    }
    const range = filter.getDateRange('created_at');
    const [start, end] = range;

    if (start === null || end === null) {
        // if we don't have a range, we can't really determine if it's safe
        // TODO: we could check the month query and see if it's within a reasonable range
        return interval === "month";
    }

    const diff = end.getTime() - start.getTime();

    const maxItems = 1000;

    switch (interval) {
        case "month":
            return diff <= maxItems * 30 * 24 * 60 * 60 * 1000;
        case "day":
            return diff <= maxItems * 24 * 60 * 60 * 1000;
        case "hour":
            return diff <= maxItems * 60 * 60 * 1000;
    }
}