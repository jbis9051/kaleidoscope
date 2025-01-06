import {useEffect, useState} from "react";
import {FilterOps} from "@/hooks/useQueryState";
import styles from "@/components/FilterPanel.module.css";

type FilterInputOps = {
    [P in keyof FilterOps]: string | null;
};

interface FilterPanelProps {
    filter: FilterOps;

    trashEnabled: boolean;

    setFilter: (filter: FilterOps) => void;
    onTrash: () => void;
    onSave: () => void;
}

export default function FilterPanel({filter, trashEnabled, setFilter, onTrash, onSave}: FilterPanelProps) {
    const [filterInput, setFilterInput] = useState<FilterInputOps>({path: null, before: null, after: null});

    // update the filterInputs when the filter changes
    useEffect(() => {
        setFilterInput({
            path: filter.path,
            before: filter.before?.toISOString().split('T')[0] || null,
            after: filter.after?.toISOString().split('T')[0] || null
        })
    }, [filter])

    return (
        <div className={styles.filterPanel}>
            <div className={styles.filterHeader}>
                <div className={styles.filterTitle}>Filters</div>
                <div>
                    <button onClick={onSave}>Save</button>
                    <button disabled={!trashEnabled} onClick={onTrash}>Trash</button>
                    <button onClick={() => {
                        setFilter({
                            path: filterInput.path,
                            before: filterInput.before ? new Date(filterInput.before) : null,
                            after: filterInput.after ? new Date(filterInput.after) : null,
                        });
                    }}>Filter
                    </button>
                </div>
            </div>
            <div className={styles.filter}>
                <label>
                    <span>Path </span> <input value={filterInput.path || ''} onChange={e => {
                    setFilterInput({...filterInput, path: e.target.value})
                }} type="text" placeholder="Path Filter"/>
                </label>
                <label className={styles.filterDate}>
                    <span>Before </span> <input value={filterInput.before || ''}
                                                onChange={e => {
                                                    setFilterInput({
                                                        ...filterInput,
                                                        before: e.target.value
                                                    })
                                                }} type="date"/>
                </label>
                <label className={styles.filterDate}>
                    <span>After </span> <input value={filterInput.after || ''}
                                               onChange={e => {
                                                   setFilterInput({
                                                       ...filterInput,
                                                       after: e.target.value
                                                   })
                                               }} type="date"/>
                </label>

            </div>
        </div>
    )
}