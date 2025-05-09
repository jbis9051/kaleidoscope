import { useState, useEffect } from "react";
import Filter, {OrderByColumns} from "@/utility/Filter";


export interface QueryState {
    page: number;
    orderby: OrderByColumns;
    asc: boolean;
    limit: number;
    filter: Filter;
}

export function useQueryState(defaultState: QueryState): [QueryState, (newState: Partial<QueryState>) => void, (query: URLSearchParams) => QueryState] {
    const [state, setState] = useState<QueryState>(defaultState);

    // parse the URL query string into a state object
    function queryToState(query: URLSearchParams): QueryState {
        const page = query.get('page');
        const asc = query.get('asc');
        const limit = query.get('limit');

        const orderby = query.get('order_by');

        const filter = query.get('filter');

        return {
            page: page ? parseInt(page, 10) : defaultState.page,
            orderby: orderby || defaultState.orderby,
            asc: asc ? asc === 'true' : defaultState.asc,
            limit: limit ? parseInt(limit, 10) : defaultState.limit,
            filter: filter ? Filter.fromString(filter) : defaultState.filter,
        }
    }

    // update the URL query string when the state changes
    useEffect(() => {
        const query = new URLSearchParams();

        query.set('page', state.page.toString());
        query.set('order_by', state.orderby);
        query.set('asc', state.asc.toString());
        query.set('limit', state.limit.toString());

        if (state.filter) {
            query.set('filter', state.filter.toFilterString());
        }

        window.history.replaceState({}, '', `${window.location.pathname}?${query.toString()}`);
    }, [state]);

    const updateState = (newState: Partial<QueryState>) => {
        setState(prevState => ({ ...prevState, ...newState }));
    };

    return [state, updateState, queryToState];
}