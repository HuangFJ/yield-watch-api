import React from 'react';
import styles from './index.less'
import { Icon } from 'antd-mobile';

const Error = () => (
    <div className="content-inner">
        <div className={styles.error}>
            <Icon type="frown-o" />
            <h1>404 Not Found</h1>
        </div>
    </div>
);

export default Error;