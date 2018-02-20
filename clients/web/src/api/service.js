import request from '../lib/request'

var AUTH = {
    access_token: ''
};

function auth(args) {
    AUTH = { ...AUTH, ...args };
}

function sms(mobile) {
    return request({
        url: '/sms',
        method: 'POST',
        data: {
            'mobile': mobile
        }
    });
}

function sms_auth(mobile, code) {
    return request({
        url: '/sms/auth',
        method: 'POST',
        data: {
            'mobile': mobile,
            'code': parseInt(code)
        }
    });
}

function me_get() {
    const { access_token } = AUTH;
    return request({
        url: `/me?access_token=${access_token}`,
        method: 'GET'
    });
}

const Service = {
    auth, sms, sms_auth, me_get
};

export default Service;