import {useEffect, useState} from "react";
import styles from "@/components/FilterPanel.module.css";
import Filter from "@/utility/Filter";
import {MediaQueryDescription} from "@/api/api";

interface FilterPanelProps {
    filter: Filter;

    trashEnabled: boolean;

    setFilter: (filter: Filter) => void;
    onTrash: () => void;
    onSave: () => void;
}

export default function FilterPanel({filter, trashEnabled, setFilter, onTrash, onSave}: FilterPanelProps) {
    const [filterInput, setFilterInput] = useState<string>("");
    
    //const [description, setDescription] = useState<MediaQueryDescription | null>(null);
    
    // update the filterInputs when the filter changes
    useEffect(() => {
        setFilterInput(filter.toFilterString());
    }, [filter])

    let filterError = null;

    try {
        Filter.fromString(filterInput);
    } catch (e: any) {
        filterError = e.message;
    }

    return (
        <div className={styles.filterPanel}>
            <div className={`${styles.filter} ${filterError ? styles.error : ""}`}>
                <input placeholder={"Filter"} value={filterInput} onChange={(e) => setFilterInput(e.target.value)}/>
            </div>
            <div className={styles.actions}>
                <div>
                    <button onClick={onSave}>Save</button>
                    <button disabled={!trashEnabled} onClick={onTrash}>Trash</button>
                    <button onClick={() => setFilter(Filter.fromString(filterInput)) }>Filter</button>
                </div>
            </div>
        </div>
    )
}