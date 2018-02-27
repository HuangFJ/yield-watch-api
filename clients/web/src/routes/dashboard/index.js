import React from 'react';
import { connect } from 'dva';
import PropTypes from 'prop-types';
import styles from './index.less';

const Dashboard = ({ dashboard, loading }) => {
    return (
        <div>hi</div>
    )
}

Dashboard.propTypes = {
    dashboard: PropTypes.object,
    loading: PropTypes.object,
}

export default connect(({ dashboard, loading }) => (dashboard, loading))(Dashboard);