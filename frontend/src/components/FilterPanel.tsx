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
    const [filterInput, setFilterInput] = useState<FilterInputOps>({
        path: null,
        before: null,
        after: null,
        not_path: null,
        is_screenshot: null,
        import_id: null,
        has_gps: null
    });

    // update the filterInputs when the filter changes
    useEffect(() => {
        setFilterInput({
            path: filter.path,
            not_path: filter.not_path,
            before: filter.before?.toISOString().split('T')[0] || null,
            after: filter.after?.toISOString().split('T')[0] || null,
            is_screenshot: filter.is_screenshot?.toString() || "any",
            import_id: filter.import_id?.toString() || null,
            has_gps: filter.has_gps?.toString() || null
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
                            not_path: filterInput.not_path,
                            before: filterInput.before ? new Date(filterInput.before) : null,
                            after: filterInput.after ? new Date(filterInput.after) : null,
                            is_screenshot: filterInput.is_screenshot === 'any' ? null : filterInput.is_screenshot === 'true',
                            import_id: filter.import_id,
                            has_gps: filter.has_gps,
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
                <label>
                    <span>Not Path </span> <input value={filterInput.not_path || ''} onChange={e => {
                    setFilterInput({...filterInput, not_path: e.target.value})
                }} type="text" placeholder="Not Path Filter"/>
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
                <label>
                    <span>Is Screenshot </span>
                    <select value={filterInput.is_screenshot || 'null'} onChange={e => {
                        setFilterInput({...filterInput, is_screenshot: e.target.value})
                    }}>
                        <option value="any">Any</option>
                        <option value="true">Yes</option>
                        <option value="false">No</option>
                    </select>
                </label>

            </div>
        </div>
    )
}