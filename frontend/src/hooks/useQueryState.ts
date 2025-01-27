import { useState, useEffect } from "react";
import {MediaQueryColumns, MediaQueryColumnsType} from "@/api/api";

export class FilterOps {
    path: string | null = null;
    not_path: string | null = null;
    before: Date | null = null;
    after: Date | null = null;
    is_screenshot: boolean | null = null;
    import_id: number | null = null;
    has_gps: boolean | null = null;
}

export interface QueryState {
    page: number;
    orderby: MediaQueryColumnsType;
    asc: boolean;
    limit: number;
    selectedAlbum: string | null;
    filter: FilterOps;
}

export function useQueryState(defaultState: QueryState): [QueryState, (newState: Partial<QueryState>) => void, (query: URLSearchParams) => QueryState] {
    const [state, setState] = useState<QueryState>(defaultState);

    // parse the URL query string into a state object
    function queryToState(query: URLSearchParams): QueryState {
        const page = query.get('page');
        const orderby = query.get('orderby');
        const asc = query.get('asc');
        const limit = query.get('limit');
        const selectedAlbum = query.get('album');
        const filter_path = query.get('filter_path');
        const filter_not_path = query.get('filter_not_path');
        const before = query.get('before');
        const after = query.get('after');
        const is_screenshot = query.get('is_screenshot');
        const import_id = query.get('import_id');
        const has_gps = query.get('has_gps');

        const newFilter = new FilterOps();

        if (filter_path) {
            newFilter.path = filter_path;
        }
        if (filter_not_path) {
            newFilter.not_path = filter_not_path;
        }
        if (before) {
            newFilter.before = new Date(parseInt(before, 10));
        }
        if (after) {
            newFilter.after = new Date(parseInt(after, 10));
        }

        if (orderby && MediaQueryColumns.indexOf(orderby) === -1) {
            throw new Error(`Invalid orderby value: ${orderby}`);
        }

        if (is_screenshot) {
            newFilter.is_screenshot = is_screenshot === 'true';
        }

        if (import_id) {
            newFilter.import_id = parseInt(import_id, 10);
        }

        if (has_gps) {
            newFilter.has_gps = has_gps === 'true';
        }

        return {
            page: page ? parseInt(page, 10) : defaultState.page,
            orderby: orderby || defaultState.orderby,
            asc: asc ? asc === 'true' : defaultState.asc,
            limit: limit ? parseInt(limit, 10) : defaultState.limit,
            selectedAlbum: selectedAlbum || defaultState.selectedAlbum,
            filter: newFilter
        }
    }

    // update the URL query string when the state changes
    useEffect(() => {
        const query = new URLSearchParams();

        query.set('page', state.page.toString());
        query.set('orderby', state.orderby);
        query.set('asc', state.asc.toString());
        query.set('limit', state.limit.toString());

        if (state.selectedAlbum) {
            query.set('album', state.selectedAlbum);
        }
        if (state.filter.path) {
            query.set('filter_path', state.filter.path);
        }
        if (state.filter.not_path) {
            query.set('filter_not_path', state.filter.not_path);
        }
        if (state.filter.before) {
            query.set('before', state.filter.before.getTime().toString(10));
        }
        if (state.filter.after) {
            query.set('after', state.filter.after.getTime().toString(10));
        }
        if (state.filter.is_screenshot !== null && state.filter.is_screenshot !== undefined) {
            query.set('is_screenshot', state.filter.is_screenshot.toString());
        }
        if(state.filter.import_id !== null){
            query.set('import_id', state.filter.import_id.toString());
        }
        if (state.filter.has_gps !== null && state.filter.has_gps !== undefined) {
            query.set('has_gps', state.filter.has_gps.toString());
        }

        window.history.replaceState({}, '', `${window.location.pathname}?${query.toString()}`);
    }, [state]);

    const updateState = (newState: Partial<QueryState>) => {
        setState(prevState => ({ ...prevState, ...newState }));
    };

    return [state, updateState, queryToState];
}