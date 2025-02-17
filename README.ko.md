# 📧 AWS SES 이메일 발송기

[한국어](README.ko.md) | [English](README.md)

AWS SES와 SNS를 활용한 고성능 대량 이메일 발송 및 모니터링 서버입니다.
Rust와 Tokio를 기반으로 구축되어 높은 처리량과 안정성을 제공합니다.

## 🏗 시스템 아키텍처

### 기술 스택
- 🦀 **Backend**: Rust + Axum
- 📨 **Email Service**: AWS SES
- 🔔 **Notification**: AWS SNS
- 🔄 **Async Runtime**: Tokio
- 💾 **Database**: SQLite

### 동작 방식

#### 즉시 발송 프로세스
1. API 요청 수신 (`/v1/messages`)
2. 데이터베이스에 발송 요청 저장
3. Tokio 채널을 통해 발송기로 즉시 전달
4. AWS SES 발송 속도 제한에 맞춰 순차 처리
5. 발송 결과 비동기 저장

#### 예약 발송 프로세스
1. API 요청 수신 (scheduled_at 포함)
2. 데이터베이스에 예약 정보 저장
3. 스케줄러가 1분 주기로 예약된 작업 확인
4. 발송 시간이 된 메일을 발송기로 전달
5. 즉시 발송과 동일한 프로세스로 처리

### 성능 최적화
- Tokio 기반 비동기 런타임 활용
- 경량 스레드로 리소스 사용 최소화
- 채널 기반 효율적 태스크 분배
- AWS SES 발송 속도 제한 자동 준수

## ✨ 주요 기능

- 🚀 대량 이메일 발송 및 예약 발송
- 📊 실시간 발송 결과 모니터링
- 👀 이메일 열람 추적
- ⏸ 대기 중인 이메일 발송 취소
- 📈 발송 통계 및 결과 분석

## 🔧 설정 가이드

### AWS SES 설정하기

#### 1️⃣ 샌드박스 모드 해제 (프로덕션 환경)
- AWS SES는 기본적으로 샌드박스 모드로 시작
- 프로덕션 환경을 위해 [AWS Support Center에서 샌드박스 해제 요청](https://docs.aws.amazon.com/ses/latest/dg/request-production-access.html) 필요

#### 2️⃣ 도메인 인증
- AWS SES 콘솔에서 도메인 등록
- DNS에 DKIM, SPF 레코드 추가 (제공된 레코드 사용)
- 인증 완료까지 최대 72시간 소요

#### 3️⃣ 이메일 주소 인증 (샌드박스 모드)
- AWS SES 콘솔에서 발신자 이메일 등록
- 인증 이메일의 확인 링크로 인증 완료

### AWS SNS 설정하기

#### 1️⃣ SNS 주제 생성
- AWS SNS 콘솔에서 새 주제 생성
- 알림을 받을 주제 이름 설정

#### 2️⃣ SES 이벤트 설정
- SES Configuration Sets에서 새 설정 생성
- SNS 이벤트 대상 추가
    - 이벤트: Bounce, Complaint, Delivery
    - 생성한 SNS 주제 연결

#### 3️⃣ SNS 구독 설정
- SNS 주제에 구독 추가 (HTTP/HTTPS, Email, SQS)
- 구독 확인 절차 완료
    - HTTP/HTTPS: 엔드포인트에서 확인 요청 처리
    - Email: 확인 링크 클릭

## ⚙️ 환경 변수

```env
# AWS 설정
AWS_REGION=ap-northeast-2
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_SES_FROM_EMAIL=your_verified_email

# 서버 설정
SERVER_URL=http://localhost:3000
SERVER_PORT=3000
DATABASE_URL=sqlite://sqlite3.db
JWT_SECRET=your_secret_key
MAX_SEND_PER_SECOND=12

SENTRY_DSN=your_sentry_dsn
```

## 📡 API 가이드

### 이메일 발송

```http
POST /v1/messages
```

대량 이메일 발송 및 예약 발송을 처리합니다.

```json
{
  "messages": [
    {
      "topic_id": "newsletter_2024_01",  // 커스텀 식별자
      "emails": ["user@example.com"],
      "subject": "1월 뉴스레터",
      "content": "안녕하세요..."  // HTML 형식
    }
  ],
  "scheduled_at": "2024-01-01 09:00:00"  // 선택사항
}
```

### 발송 결과 추적

#### 📨 SNS 이벤트 수신
```http
POST /v1/events/results
```
AWS SES로부터 실시간 발송 결과를 수신합니다:
- ✅ Delivery: 발송 성공
- ❌ Bounce: 발송 실패
- ⚠️ Complaint: 스팸 신고

#### 👁 이메일 열람 확인
```http
GET /v1/events/open?request_id={request_id}
```
1x1 투명 이미지를 통해 이메일 열람 여부를 추적합니다.
- 이메일 본문 하단에 자동 포함
- 이메일 열람 시 자동으로 서버에 기록
- request_id로 개별 수신자 확인 가능

### 모니터링 & 관리

#### 📊 발송 한도 확인
```http
GET /v1/events/counts/sent
```
AWS SES 일일 발송 한도 및 잔여 수량을 확인합니다.

#### 📈 토픽별 결과 조회
```http
GET /v1/topics/{topic_id}
```
topic_id 기준으로 발송 결과를 집계합니다:
- 총 발송 수
- 성공/실패 수
- 열람 수

#### ⏹ 발송 취소
```http
DELETE /v1/topics/{topic_id}
```
대기 중인 이메일 발송을 취소합니다.
- topic_id에 해당하는 모든 대기 메일 취소
- 이미 발송된 메일은 취소 불가

## 📚 참고 자료

- [AWS SES 개발자 가이드](https://docs.aws.amazon.com/ses/latest/dg/Welcome.html)
- [AWS SNS 개발자 가이드](https://docs.aws.amazon.com/sns/latest/dg/welcome.html)