import { my_coins, my_values } from '../services/api';

export default {

    namespace: 'dashboard',

    subscriptions: {

        setup({ dispatch, history }) {
            history.listen(({ pathname }) => {
                if (pathname === '/dashboard' || pathname === '/' ) {
                    dispatch({ type: 'myCoins' });
                    dispatch({ type: 'myValues' });
                }
            });
        },

    },

    state: { },

    effects: {

        *myCoins(_, { call, put }) {
            const data = yield call(my_coins);
            console.log(data);
        },

        *myValues(_, { call, put }) {
            const data = yield call(my_values);
            console.log(data);
        },
        
    },

    reducers: { },

}