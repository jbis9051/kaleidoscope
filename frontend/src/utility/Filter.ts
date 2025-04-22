export type OrderByColumns = string;
export type Value = any;

type FilterType = { [key: string]: [string, Value][] };

export default class Filter {
    filter: FilterType = {};

    static fromString(filter: string): Filter {
        const f = new Filter();

        if (filter.length === 0) {
            return f;
        }

        const filters = Filter.parseFilter(filter);

        for (const filter of filters) {
            const [key, rest] = filter.split(':');

            if (!rest) {
                throw new Error(`invalid filter '${filter}'`);
            }

            const match = rest.match(/((?:[><!]=?)|%|=)(.+)/);
            if (!match || match.length !== 3) {
                throw new Error(`invalid filter '${filter}'`);
            }

            const [op, value] = match.splice(1);

            f.set(key, op, value);
        }

        return f;
    }

    static parseFilter(query_string: string): string[] {
        const chars = Array.from(query_string);
        const filters: string[] = [];

        let quote: string | null = null;
        let curr = 0;
        let currFilter = "";

        while (curr < chars.length) {
            const c = chars[curr];
            switch (true) {
                case /\s/.test(c):
                    if (quote === null) {
                        if (currFilter.length > 0) {
                            filters.push(currFilter);
                            currFilter = "";
                        }
                    } else {
                        currFilter += c;
                    }
                    break;
                case c === "\\":
                    if (curr + 1 >= chars.length) {
                        throw new Error("unexpected end of string");
                    }
                    currFilter += chars[curr + 1];
                    curr += 1;
                    break;
                case c === '"' || c === "'":
                    if (quote !== null){
                        if (quote === c) {
                            quote = null;
                        } else {
                            currFilter += c;
                        }
                    } else {
                        quote = c;
                    }
                    break;
                default:
                    currFilter += c;
                    break;
            }
            curr += 1;
        }

        if (quote !== null) {
            throw new Error(`unmatched quote: ${quote}`);
        }

        if (currFilter.length > 0) {
            filters.push(currFilter);
        }

        return filters;
    }

    toFilterString(): string {
        // <key>:<op>value> <key>:<op><value>
        return Filter.stringify(this.filter);
    }

    private static stringify(filters: FilterType) {
        return Object.entries(filters).map(([key, values]) => {
            return values.map(([op, value]) => `${key}:${op}${Filter.valueToString(value)}`).join(' ');
        }).join(' ');
    }

    private static valueToString(value: Value): string {
        if (typeof value === 'string') {
            const includesDoubleQuotes = value.includes('"');
            const includesSingleQuotes = value.includes("'");
            const includesWhitespace = value.includes(' ');

            switch (true) {
                case !includesDoubleQuotes:
                    return `"${value}"`;
                case !includesSingleQuotes:
                    return `'${value}'`;
                default:
                    return `"${value.replace(/"/g, '\\"')}"`;
            }
        }
        return value.toString();
    }

    toMediaQuery(order_by: OrderByColumns, asc: boolean, limit: number, page: number) {
        const filterString = this.toFilterString();
        return `${filterString} order_by:=${order_by} asc:=${asc} limit:=${limit} page:=${page}`;
    }

    static empty() {
        return new Filter();
    }

    get<T=Value>(key: string, op: string): T | null {
        const values = this.filter[key];
        if (!values) {
            return null;
        }

        const value = values.find(([op_, _]) => op_ === op);
        if (!value) {
            return null;
        }

        return value[1];
    }

    set(key: string, op: string, value: Value | null) {
        if (!this.filter[key]) {
            this.filter[key] = [];
        }
        this.filter[key] = this.filter[key].filter(([op_, _]) => op_ !== op);
        if(value !== null){
            this.filter[key].push([op, value]);
        }
        return this;
    }

    add(key: string, op: string, value: Value) {
        if (!this.filter[key]) {
            this.filter[key] = [];
        }
        this.filter[key].push([op, value]);
        return this;
    }

    clone() {
       return Filter.fromString(this.toFilterString());
    }

    equals(other: Filter) {
        const keys = Object.keys(this.filter);
        const otherKeys = Object.keys(other.filter);
        if (keys.length !== otherKeys.length) {
            return false;
        }
        keys.sort();
        otherKeys.sort();
        if (keys.join('|') !== otherKeys.join('|')) {
            return false;
        }
        for (const key of keys) {
            const values = this.filter[key];
            const otherValues = other.filter[key];
            if (values.length !== otherValues.length) {
                return false;
            }
            for (let i = 0; i < values.length; i++) {
                const [op, value] = values[i];
                const [otherOp, otherValue] = otherValues[i];
                if (op !== otherOp || value !== otherValue) {
                    return false;
                }
            }
        }
        return true;
    }

    getDateRange(key: string){
        let max = null;
        let min = null;


        const maxes = [this.get(key, "<="), this.get(key, "<")].filter(v => v !== null);
        if (maxes.length > 0) {
            max = new Date(maxes.map(d => new Date(d)).reduce((a, b) => a > b ? a : b, new Date(-8640000000000000)));
        }

        const mins = [this.get(key, ">="), this.get(key, ">")].filter(v => v !== null);
        if (mins.length > 0) {
            min = new Date(mins.map(d => new Date(d)).reduce((a, b) => a < b ? a : b, new Date(8640000000000000)));
        }

        return [min, max];
    }
}